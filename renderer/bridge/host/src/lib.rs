//! The native host library of C9's product shell, step S2 of `renderer/shell/README.md` (a draft): c600-display's
//! host over the bridge, behind a C interface that the shell's C# side calls.
//!
//! Route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks: the shell loads
//! `sa2_interop.dll`, creates a context on the core's own device with `sa2_create`, attaches the surface beneath its
//! window, and hands this library the core's function table ([`CoreFunctions`]) and the context. Frames go to the
//! surface (`SA2_TARGET_SURFACE`). This library loads no library and calls no Windows function (CT-RPI-20). It starts
//! one process, the language runtime ([`m6h_load_runtime`]), through c600-runtime's parent side, which uses std's
//! portable process API, and one thread, which runs the view jobs in that process ([`view`]). With a preload
//! ([`m6h_preload`]) it also starts one thread for each model load, which ends with its load ([`preload`]).
//!
//! Drafts that the sources leave open, each to be replaced by LP3's parts:
//! - One view, which shows the adopted state as one retained publication of every slot, visible and interactive.
//!   There is no picking or preview yet.
//! - The adopted state starts as the shipped model's solved state at engine revision [`ENGINE_REVISION`]. With a
//!   session open ([`m6h_open_session`], charter E2), each twist key the shell routes ([`m6h_key_down`]) that commits
//!   the puzzle's twist row (the [`turns`] module) adopts the layer's new head at the next engine revision and
//!   publishes the view again for it, so the view's output is never stale (RPI.13) for longer than its preparation
//!   takes. A frame shows a press once its engine revision and its publication's revision have both reached the
//!   press's revision.
//! - The jumbled puzzle (LX-4's jumble-on half; a draft): a session opened with [`M6H_PUZZLE_JUMBLE_ON`] commits
//!   `jumble.on` at its start, and the host adopts the poses at the next engine revision as c600-display's posed row
//!   (J02: `jumble::posed` and `jumble::bundle`), with one posed primitive in the view's plan ([`adoption`]). The host
//!   context becomes c600-display's declared jumbling context for the menu's identity block, with the core's own
//!   support report. c600-render's bridge hands the core the bundle's pose arrays over the home labels, so each
//!   sticker keeps its colour and moves with its piece's pose, and draws every slot of the posed plan, none pickable.
//! - Charter E3 (a draft): the session may instead be an existing one, a copy of a session fixture ([`fixture`]),
//!   which the host adopts at the engine revision that counts the fixture's presses. [`m6h_load_runtime`] starts the
//!   language runtime's process and keeps it until the handle is destroyed. [`m6h_preload`] starts the model's loads
//!   before the shell has a core context, and [`m6h_create_preloaded`] and [`m6h_open_session`] join them.
//! - View jobs (C9 U8; the [`view`] module): [`m6h_run_view`] schedules a program text as the view's job and runs it
//!   in the runtime on a worker thread. A run that completes delivers the reference publication of the revision it
//!   was scheduled at, with the plan dependency `program` set to the hex SHA-256 of its text; its value is not drawn
//!   yet. The run serves no host row, its limits [`VIEW_LIMITS`], deadline [`VIEW_DEADLINE`] and output allowance
//!   [`VIEW_ALLOWANCE`] are declared values, and three refusal codes are the host's own (`m6h:` codes, not in chapter
//!   13's register). [`m6h_view_draft`] names the editor's digest without a run, so that the view shows stale output
//!   while the text differs from the one its publication came from. [`m6h_view_result`] reads a ticket's record:
//!   status, value digest, and the first confirmed frames that drew the run's publication or the draft's stale and
//!   current output.
//! - The host context is c600-display's declared fixture context, with the core's own support report in place of the
//!   declared one. Its tokens, words and stage signatures are fixtures, not the checked library's definitions, and the
//!   limit profile [`LIMITS`] is a declared value, not a measured one.
//! - Engine adoption goes through the host alone: `Host::adopt` validates the bundle and hands it to c600-render's
//!   bridge through c600-display's `Backend::adopt_engine`, and records the revision only once the core holds it.
//! - After a recovery, c600-display's `Host::recover` prepares the installed publication again for the new device
//!   when it is the library's newest one (RM.23). Otherwise the newest one was never installed (published or adopted
//!   while the device was lost), and the library publishes it again.
//!
//! Calls: a handle is used by one thread at a time, as the core's context is (a `sa2_create` context takes no lock,
//! and the caller serialises). Every function checks its pointers, catches a panic and answers a status code. A panic
//! poisons the handle: it then answers only [`M6H_E_POISONED`] until it is destroyed. Nothing here is Windows/DirectX,
//! input or performance evidence.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Arc;
use std::time::Duration;

use c600_display::array::Array;
use c600_display::codes;
use c600_display::host::{Backend, Host};
use c600_display::jumble;
use c600_display::pick::PresentedFrame;
use c600_display::plan::*;
use c600_display::produce::{self, full_set, GeometryInput};
use c600_display::validate::{BoundArray, EngineBundle, LimitProfile};
use c600_display::{Error, Status as CommandStatus};
use c600_kernel::json::Value;
use c600_kernel::{Kernel, LoadError, Snapshot};
use c600_render::abi::TARGET_SURFACE;
use c600_render::{DisplayTime, RenderBackend};
pub use c600_render_ffi::CoreFunctions;
use c600_render_ffi::DllCore;

pub mod fixture;
pub mod preload;
pub mod turns;
pub mod view;
use c600_run::meter::Amounts;
use c600_runtime::parent::{Config as RuntimeConfig, Runtime};
use preload::{ExactLoad, Preload};
use turns::{Head, Press, Puzzle, Turns};

/// The version of this C interface. The shell refuses a library that answers another. Version 2 added the frame's
/// revisions and the session calls; version 3 the session fixture and the runtime; version 4 the view jobs; version 5
/// the session's puzzle; version 6 the preload.
pub const M6H_ABI_VERSION: u32 = 6;

pub const M6H_OK: i32 = 0;
/// A null or malformed argument.
pub const M6H_E_INVALID_ARGUMENT: i32 = 1;
/// The shipped model did not load from the assets folder.
pub const M6H_E_MODEL: i32 = 2;
/// The core or the bridge refused to bind the context.
pub const M6H_E_CORE: i32 = 3;
/// The display host refused; [`m6h_last_error`] gives the reason.
pub const M6H_E_HOST: i32 = 4;
/// The renderer is unavailable, for example after the device was lost; [`m6h_recover`] may bring it back.
pub const M6H_E_UNAVAILABLE: i32 = 5;
/// An earlier call panicked; only [`m6h_last_error`] and [`m6h_destroy`] are left.
pub const M6H_E_POISONED: i32 = 6;
/// The session did not open, is already open, or is not open; [`m6h_last_error`] gives the reason.
pub const M6H_E_SESSION: i32 = 7;
/// The language runtime did not start, or the handle has started one already; [`m6h_last_error`] gives the reason.
pub const M6H_E_RUNTIME: i32 = 8;

/// [`M6hKey::press`]: the key-down was no measured press (no twist key, or a repeat).
pub const M6H_PRESS_NONE: u32 = 0;
/// [`M6hKey::press`]: a twist key committed the puzzle's twist row: `twist.apply`, or `jumble.twist.menu` with Jumble
/// on.
pub const M6H_PRESS_COMMITTED: u32 = 1;
/// [`M6hKey::press`]: a twist key that the router or the layer refused; the head did not change.
pub const M6H_PRESS_REFUSED: u32 = 2;
/// [`M6hKey::press`]: no measured press, but another row changed the state (the draft map's `session.undo` and
/// `session.redo`). The host adopts and shows the new state; the correctness control then fails, since its replay
/// knows only twists.
pub const M6H_PRESS_OTHER: u32 = 3;

/// [`m6h_open_session`]'s `puzzle`: 600-cell-Full with Jumble off, the shell's `jumble-off` (index 0 of its
/// `EventLog.Puzzles`).
pub const M6H_PUZZLE_JUMBLE_OFF: u32 = 0;
/// [`m6h_open_session`]'s `puzzle`: the jumbled puzzle, the shell's `jumble-on` (index 1 of its `EventLog.Puzzles`).
pub const M6H_PUZZLE_JUMBLE_ON: u32 = 1;

/// [`M6hViewResult::status`]: the run has not ended yet.
pub const M6H_VIEW_STATUS_PENDING: u32 = 0;
/// [`M6hViewResult::status`]: the run's publication was installed; a draft is recorded at once.
pub const M6H_VIEW_STATUS_PUBLISHED: u32 = 1;
/// [`M6hViewResult::status`]: the run ended without a publication; [`m6h_last_error`] gives its code and reason.
pub const M6H_VIEW_STATUS_REFUSED: u32 = 2;
/// [`M6hViewResult::kind`]: a run of [`m6h_run_view`].
pub const M6H_VIEW_KIND_RUN: u32 = 0;
/// [`M6hViewResult::kind`]: a draft of [`m6h_view_draft`].
pub const M6H_VIEW_KIND_DRAFT: u32 = 1;

/// The one view of the draft.
pub const VIEW: ViewId = ViewId(1);
/// The engine revision of the adopted state.
pub const ENGINE_REVISION: u64 = 1;
/// The declared limit profile of the draft, not a measured one.
pub const LIMITS: &str = "shell-draft/1";
/// A view job's run limits: declared draft values, not measured ones.
pub const VIEW_LIMITS: Amounts = Amounts { steps: 1 << 32, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };
/// A view job's wall-clock safety stop (KN7.10), a draft value.
pub const VIEW_DEADLINE: Duration = Duration::from_secs(60);
/// The output allowance of a view job's publication in canonical bytes, a draft value: 16 MiB, about ten times the
/// 1,561,680 bytes that the reference publication of the shipped model with a `program` dependency measured on
/// 19 Nov 2026.
pub const VIEW_ALLOWANCE: u64 = 1 << 24;

const IDENTITY: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// A submitted frame, as [`m6h_frame`] writes it: the fields of charter E2's `frame` row that the host knows.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M6hFrame {
    /// The host's frame sequence: the frame's id in the shell's logs.
    pub sequence: u64,
    pub device_epoch: u64,
    pub surface: u64,
    /// The engine revision whose labels the frame draws (the stamp's).
    pub engine_revision: u64,
    /// The engine revision of the publication the frame draws, or 0 when it draws none.
    pub publication_revision: u64,
}

/// What [`m6h_key_down`] did with one key-down.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M6hKey {
    /// [`M6H_PRESS_NONE`], [`M6H_PRESS_COMMITTED`], [`M6H_PRESS_REFUSED`] or [`M6H_PRESS_OTHER`].
    pub press: u32,
    pub reserved: u32,
    /// The adopted engine revision after the key: for a committed press, the revision that shows it.
    pub engine_revision: u64,
}

/// One confirmed presentation (RM.26), as [`m6h_read_presented`] writes it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M6hPresented {
    /// The host's frame sequence, as [`m6h_frame`] answered it.
    pub sequence: u64,
    pub device_epoch: u64,
    pub surface: u64,
    pub engine_revision: u64,
    /// The camera version the frame drew, as [`m6h_set_camera`] counts it.
    pub camera: u64,
    /// The SyncQPCTime of the statistics row that confirmed the frame, in QPC ticks (charter E2's display time).
    pub display_qpc: u64,
}

/// A view job's record, as [`m6h_view_result`] writes it. 104 bytes; `reserved` keeps the layout free of padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M6hViewResult {
    /// [`M6H_VIEW_STATUS_PENDING`], [`M6H_VIEW_STATUS_PUBLISHED`] or [`M6H_VIEW_STATUS_REFUSED`].
    pub status: u32,
    /// [`M6H_VIEW_KIND_RUN`] or [`M6H_VIEW_KIND_DRAFT`].
    pub kind: u32,
    /// The engine revision adopted at the call: the revision a run's publication shows.
    pub revision: u64,
    /// The SHA-256 of the run's source text, or the draft's digest.
    pub source_digest: [u8; 32],
    /// A run's: the first confirmed frame that drew its publication. A draft's: the first confirmed frame after the
    /// draft that drew stale output. 0 until known.
    pub frame: u64,
    /// A draft's: the first confirmed frame after `frame` whose output was no longer stale; 0 until known.
    pub cleared_frame: u64,
    /// 1 when the run completed with a value.
    pub has_value: u32,
    pub reserved: u32,
    /// The SHA-256 of the completed run's value bytes (its canonical output unit).
    pub value_sha256: [u8; 32],
}

const _: () = assert!(std::mem::size_of::<M6hViewResult>() == 104);

impl M6hPresented {
    /// The row of `frame`, which the core confirmed at `time`; both come from one read of the core's rows.
    fn new(frame: &PresentedFrame, time: &DisplayTime) -> Result<M6hPresented, Failure> {
        let s = &frame.stamp;
        if (time.sequence, time.device_epoch, time.surface) != (s.sequence, s.device_epoch, s.surface) {
            return Err((M6H_E_HOST, "a display time names another frame than its presentation".to_owned()));
        }
        Ok(M6hPresented {
            sequence: s.sequence,
            device_epoch: s.device_epoch,
            surface: s.surface,
            engine_revision: s.engine_revision,
            camera: s.camera,
            display_qpc: time.display_qpc,
        })
    }
}

/// What a handle points to.
pub struct ShellHost {
    host: Host<RenderBackend<DllCore>>,
    kernel: Arc<Kernel>,
    geometry: GeometryInput,
    assets: PathBuf,
    /// The exact model's load that [`m6h_preload`] started for the session, until [`m6h_open_session`] takes it.
    exact: Option<ExactLoad>,
    publication: Publication,
    next_command: u64,
    /// The adopted engine revision, and each adopted revision's state digest.
    revision: u64,
    digests: BTreeMap<u64, String>,
    turns: Option<Turns>,
    /// The correctness control's record, once [`m6h_close_session`] made it.
    check: String,
    /// The language runtime's process and its program, once [`m6h_load_runtime`] started it and until the first
    /// view job moves it to the worker; dropping it ends the process.
    runtime: Option<(Runtime, PathBuf)>,
    /// The command of the newest reference publication ([`ShellHost::publish`]).
    reference: Option<CommandId>,
    views: view::Views,
    presented: VecDeque<M6hPresented>,
    error: String,
    poisoned: bool,
}

type Failure = (i32, String);

thread_local! {
    /// Why the last [`m6h_create`] on this thread failed.
    static CREATE_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn host_error(error: Error) -> Failure {
    let code = if error.code == codes::RENDERER_UNAVAILABLE { M6H_E_UNAVAILABLE } else { M6H_E_HOST };
    let context: Vec<String> = error.context.iter().map(|(k, v)| format!("{k}={v}")).collect();
    (code, if context.is_empty() { error.to_string() } else { format!("{error} ({})", context.join(", ")) })
}

fn model_error(error: LoadError) -> Failure {
    (M6H_E_MODEL, format!("the shipped model did not load: {}", error.0))
}

/// RM.16's engine bundle of `state`: its labels, every slot visible and interactive.
pub fn engine_bundle(state: &Snapshot, revision: &RevisionRef) -> EngineBundle {
    let set = full_set();
    let input = produce::retained(state, &set, &set, Vec::new());
    let arrays = [("labels", input.labels), ("visibility", input.visibility), ("interaction", input.interaction)];
    let bound = |(name, array): (&str, Array)| BoundArray { name: name.to_owned(), revision: revision.state, array };
    EngineBundle { revision: revision.clone(), arrays: arrays.into_iter().map(bound).collect() }
}

/// The adoption of `head` at engine revision `revision`: its revision reference, its engine bundle and the view's
/// primitive. With Jumble off they are RM.16's labels and one retained primitive of every slot, each visible and
/// interactive. With Jumble on they are J02's posed row, bound to the revision, and one posed primitive of it; a posed
/// row that lacks data answers [`M6H_E_HOST`], and nothing is built from it.
pub fn adoption(
    geometry: &GeometryInput,
    head: &Head,
    revision: u64,
) -> Result<(RevisionRef, EngineBundle, Primitive), (i32, String)> {
    match head {
        Head::Retained(state) => {
            let base = produce::revision(&geometry.identity, revision, state);
            let bundle = engine_bundle(state, &base);
            let set = full_set();
            Ok((base, bundle, Primitive::Retained(produce::retained(state, &set, &set, Vec::new()))))
        }
        Head::Jumbled(exact, state) => {
            let base = jumble::revision(exact, state, revision);
            let posed = jumble::posed(exact, state)
                .map_err(|missing| (M6H_E_HOST, format!("the posed row lacks data: {missing:?}")))?;
            let bundle = jumble::bundle(&base, &posed);
            Ok((base, bundle, Primitive::Posed(posed)))
        }
    }
}

/// The draft's publication of [`VIEW`] at `base`: one plan with the one primitive of [`adoption`], and no instances.
/// Its geometry binding names `base`'s identity block with the verified geometry's digests.
pub fn publication(geometry: &GeometryInput, primitive: Primitive, base: &RevisionRef) -> Publication {
    let plan = DisplayPlan {
        plan: PlanId(1),
        base: base.clone(),
        subject: StateRef::Snapshot(base.clone()),
        derivation: None,
        reachability: None,
        dimension: 4,
        geometry: GeometryBinding { identity: base.identity.clone(), digests: geometry.digests() },
        primitives: vec![primitive],
        charts: Vec::new(),
        instances: Vec::new(),
        source_map: MapId(1),
        precision: "binary32".to_owned(),
        dependencies: BTreeMap::from([("state".to_owned(), base.state.to_string())]),
    };
    let map =
        SourceMap { map: MapId(1), plan: PlanId(1), base: base.clone(), sources: Vec::new(), entries: BTreeMap::new() };
    Publication {
        target: VIEW,
        base: base.clone(),
        nodes: Vec::new(),
        plans: vec![plan],
        maps: vec![map],
        sources: BTreeMap::new(),
    }
}

impl ShellHost {
    fn create(core: DllCore, loads: Preload) -> Result<ShellHost, Failure> {
        let Preload { assets, kernel, geometry, exact } = loads;
        let kernel = Arc::new(kernel.join().map_err(model_error)?);
        let geometry = geometry.join().map_err(model_error)?;
        let head = Head::Retained(kernel.solved());
        let (base, bundle, primitive) = adoption(&geometry, &head, ENGINE_REVISION)?;
        let limits = LimitProfile::uniform(LIMITS, 1 << 40);
        let mut backend = RenderBackend::new(core, limits.clone());
        backend.adopt_engine(&bundle).map_err(host_error)?;
        let support = backend.describe_backend().map_err(host_error)?;
        backend.set_frame_target(TARGET_SURFACE);
        backend.set_camera_matrix(VIEW, 0, IDENTITY);
        let mut ctx = produce::fixture_context(&geometry, limits);
        ctx.support = support;
        let mut host = Host::new(backend, ctx, base.clone());
        host.open_view(VIEW).map_err(host_error)?;
        let mut shell = ShellHost {
            host,
            publication: publication(&geometry, primitive, &base),
            kernel,
            geometry,
            assets,
            exact,
            next_command: 1,
            revision: ENGINE_REVISION,
            digests: BTreeMap::from([(ENGINE_REVISION, head.digest())]),
            turns: None,
            check: String::new(),
            runtime: None,
            reference: None,
            views: view::Views::default(),
            presented: VecDeque::new(),
            error: String::new(),
            poisoned: false,
        };
        shell.publish()?;
        Ok(shell)
    }

    /// Adopts `head` at the next engine revision; see [`ShellHost::adopt`].
    fn advance(&mut self, head: &Head) -> Result<u64, Failure> {
        self.adopt(head, self.revision + 1)
    }

    /// Adopts `head` at engine revision `revision`, which must follow the adopted one, through the host, which hands it
    /// to the core and records it once the core holds it, and publishes the view for it ([`adoption`]). A head under
    /// another identity block than the host context's (the jumbled puzzle's poses after the solved labels) then gives
    /// the host that block's declared context, with the core's own support report, read before the adoption. A view
    /// job that read the old revision ends `KN7:cancelled.revision_replaced` ([`ShellHost::replace_runs`]). While the
    /// device is lost the publication waits for [`m6h_recover`], which publishes it. A host or core that refuses the
    /// bundle changes nothing.
    fn adopt(&mut self, head: &Head, revision: u64) -> Result<u64, Failure> {
        if revision <= self.revision {
            return Err((M6H_E_HOST, format!("engine revision {revision} does not follow {}", self.revision)));
        }
        let (base, bundle, primitive) = adoption(&self.geometry, head, revision)?;
        let ctx = if base.identity != self.host.ctx.identity {
            let limits = self.host.ctx.limits.clone();
            let mut ctx = match head {
                Head::Retained(_) => produce::fixture_context(&self.geometry, limits),
                Head::Jumbled(..) => jumble::fixture_context(&self.geometry, base.identity.clone(), limits),
            };
            ctx.support = self.host.backend.describe_backend().map_err(host_error)?;
            Some(ctx)
        } else {
            None
        };
        self.host.adopt(&bundle).map_err(host_error)?;
        if let Some(ctx) = ctx {
            self.host.ctx = ctx;
        }
        self.revision = revision;
        self.digests.insert(revision, head.digest());
        self.publication = publication(&self.geometry, primitive, &base);
        self.replace_runs();
        match self.publish() {
            Err((M6H_E_UNAVAILABLE, _)) | Ok(()) => Ok(revision),
            Err(failure) => Err(failure),
        }
    }

    fn turns(&mut self) -> Result<&mut Turns, Failure> {
        self.turns.as_mut().ok_or_else(|| (M6H_E_SESSION, "no session is open".to_owned()))
    }

    /// Publishes the view's publication under a new command and starts its preparation.
    fn publish(&mut self) -> Result<(), Failure> {
        let command = CommandId(self.next_command);
        self.next_command += 1;
        self.reference = Some(command);
        let receipt = self.host.publish(command, self.publication.clone());
        match receipt.status {
            CommandStatus::Refused(error) | CommandStatus::Failed(error) => Err(host_error(error)),
            _ => {
                self.host.pump();
                Ok(())
            }
        }
    }

    /// Delivers the view jobs that ended, polls the host, and closes the runs it ended.
    fn poll(&mut self) {
        self.collect_views();
        self.host.pump();
        self.settle_views();
    }
}

/// Runs `body` on a live handle and turns its outcome into a status code.
///
/// # Safety
///
/// `handle` is null or a live handle from [`m6h_create`] that no other thread is using.
unsafe fn with_host(handle: *mut ShellHost, body: impl FnOnce(&mut ShellHost) -> Result<(), Failure>) -> i32 {
    // SAFETY: the caller's contract.
    let Some(shell) = (unsafe { handle.as_mut() }) else {
        return M6H_E_INVALID_ARGUMENT;
    };
    if shell.poisoned {
        return M6H_E_POISONED;
    }
    match catch_unwind(AssertUnwindSafe(|| body(&mut *shell))) {
        Ok(Ok(())) => M6H_OK,
        Ok(Err((code, text))) => {
            shell.error = text;
            code
        }
        Err(_) => {
            shell.poisoned = true;
            shell.error = "a call panicked; the handle only answers M6H_E_POISONED now".to_owned();
            M6H_E_POISONED
        }
    }
}

/// The version of this C interface, [`M6H_ABI_VERSION`].
#[no_mangle]
pub extern "C" fn m6h_abi_version() -> u32 {
    M6H_ABI_VERSION
}

/// Binds a core context, loads the shipped model from `assets`, adopts its solved state, opens the view and starts
/// the preparation of its publication. On success `*out` is the new handle; on failure it is null and
/// [`m6h_last_error`] with a null handle gives the reason.
///
/// # Safety
///
/// `functions` points to the [`c600_render_ffi::SYMBOLS`] exports of one loaded `sa2_interop.dll`, and `context` is
/// a context that DLL created, with a surface attached; both stay valid, and the context is used by nothing else,
/// until [`m6h_destroy`]. `assets` points to `assets_len` bytes, the UTF-8 path of the assets folder; `out` points to
/// a writable handle.
#[no_mangle]
pub unsafe extern "C" fn m6h_create(
    functions: *const CoreFunctions,
    context: *mut c_void,
    assets: *const u8,
    assets_len: usize,
    out: *mut *mut ShellHost,
) -> i32 {
    if out.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: `out` is writable (the contract).
    unsafe { *out = ptr::null_mut() };
    let attempt = catch_unwind(AssertUnwindSafe(|| {
        if functions.is_null() || context.is_null() || assets.is_null() {
            return Err((M6H_E_INVALID_ARGUMENT, "a null function table, context or assets path".to_owned()));
        }
        // SAFETY: `assets` holds `assets_len` readable bytes (the contract).
        let bytes = unsafe { std::slice::from_raw_parts(assets, assets_len) };
        let path = std::str::from_utf8(bytes)
            .map_err(|_| (M6H_E_INVALID_ARGUMENT, "the assets path is not UTF-8".to_owned()))?;
        // SAFETY: the table and the context come from one loaded core and stay valid (the contract).
        let core = unsafe { DllCore::new(*functions, context) }
            .map_err(|status| (M6H_E_CORE, format!("the core context was not bound: {}", status.name())))?;
        ShellHost::create(core, Preload::deferred(Path::new(path)))
    }));
    // SAFETY: as above.
    unsafe { created(attempt, out) }
}

/// Writes a created handle to `*out`, or records why the creation failed.
///
/// # Safety
///
/// `out` points to a writable handle.
unsafe fn created(attempt: std::thread::Result<Result<ShellHost, Failure>>, out: *mut *mut ShellHost) -> i32 {
    let failure = match attempt {
        Ok(Ok(shell)) => {
            // SAFETY: the contract.
            unsafe { *out = Box::into_raw(Box::new(shell)) };
            return M6H_OK;
        }
        Ok(Err(failure)) => failure,
        Err(_) => (M6H_E_POISONED, "creation panicked".to_owned()),
    };
    CREATE_ERROR.with(|e| *e.borrow_mut() = failure.1);
    failure.0
}

/// Charter E3's startup (a draft): starts the loads that [`m6h_create`] makes from the assets folder `assets` (UTF-8,
/// `assets_len` bytes), the shipped model's kernel and its verified geometry, each on a thread of its own, and with
/// `session` 1 also the exact model's, which [`m6h_open_session`] pins the twist menu with; `session` 0 starts no
/// exact load. The call returns at once with the preload in `*out`, which [`m6h_create_preloaded`] takes and
/// [`m6h_preload_free`] frees. A load's failure is answered where the load is joined, as without a preload, and a load
/// whose thread does not start runs there.
///
/// A null or non-UTF-8 path or a `session` other than 0 or 1 answers [`M6H_E_INVALID_ARGUMENT`] and starts nothing;
/// [`m6h_last_error`] with a null handle gives the reason.
///
/// # Safety
///
/// `assets` points to `assets_len` readable bytes; `out` points to a writable preload.
#[no_mangle]
pub unsafe extern "C" fn m6h_preload(
    assets: *const u8,
    assets_len: usize,
    session: u32,
    out: *mut *mut Preload,
) -> i32 {
    if out.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: `out` is writable (the contract).
    unsafe { *out = ptr::null_mut() };
    let attempt = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the contract.
        let path = unsafe { utf8(assets, assets_len, "assets path") }?;
        let session = match session {
            0 => false,
            1 => true,
            other => return Err((M6H_E_INVALID_ARGUMENT, format!("{other} is not a session flag of this interface"))),
        };
        Ok(Preload::start(Path::new(path), session))
    }));
    let failure = match attempt {
        Ok(Ok(preload)) => {
            // SAFETY: as above.
            unsafe { *out = Box::into_raw(Box::new(preload)) };
            return M6H_OK;
        }
        Ok(Err(failure)) => failure,
        Err(_) => (M6H_E_POISONED, "the preload panicked".to_owned()),
    };
    CREATE_ERROR.with(|e| *e.borrow_mut() = failure.1);
    failure.0
}

/// [`m6h_create`] from the assets folder of `preload`, with the loads that [`m6h_preload`] started: the call joins
/// the kernel's and then the geometry's load where [`m6h_create`] makes them, and a load that failed answers as it
/// does there. An exact model's load stays with the handle until [`m6h_open_session`] takes it. The call takes
/// `preload` whatever it answers; a null `preload` answers [`M6H_E_INVALID_ARGUMENT`].
///
/// # Safety
///
/// As [`m6h_create`] for `functions`, `context` and `out`; `preload` is null or a preload from [`m6h_preload`] that
/// no call has taken or freed, and it is not used again.
#[no_mangle]
pub unsafe extern "C" fn m6h_create_preloaded(
    functions: *const CoreFunctions,
    context: *mut c_void,
    preload: *mut Preload,
    out: *mut *mut ShellHost,
) -> i32 {
    // SAFETY: the contract; the preload is this call's from here on.
    let preload = (!preload.is_null()).then(|| unsafe { Box::from_raw(preload) });
    if out.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: `out` is writable (the contract).
    unsafe { *out = ptr::null_mut() };
    let attempt = catch_unwind(AssertUnwindSafe(|| {
        let Some(preload) = preload else {
            return Err((M6H_E_INVALID_ARGUMENT, "a null preload".to_owned()));
        };
        if functions.is_null() || context.is_null() {
            return Err((M6H_E_INVALID_ARGUMENT, "a null function table or context".to_owned()));
        }
        // SAFETY: the table and the context come from one loaded core and stay valid (the contract).
        let core = unsafe { DllCore::new(*functions, context) }
            .map_err(|status| (M6H_E_CORE, format!("the core context was not bound: {}", status.name())))?;
        ShellHost::create(core, *preload)
    }));
    // SAFETY: as above.
    unsafe { created(attempt, out) }
}

/// Frees a preload that no call took. The call waits for the loads still running and drops their results.
///
/// # Safety
///
/// `preload` is null or a preload from [`m6h_preload`] that no call has taken or freed; it is not used again.
#[no_mangle]
pub unsafe extern "C" fn m6h_preload_free(preload: *mut Preload) {
    if !preload.is_null() {
        // SAFETY: the contract; a panic in a destructor is not carried across the C boundary.
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(preload) })));
    }
}

/// Sets the view's 4D rotation (16 floats, column-major, as the core's frame input takes it) as a new camera version.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `matrix` points to 16 readable floats.
#[no_mangle]
pub unsafe extern "C" fn m6h_set_camera(handle: *mut ShellHost, matrix: *const f32) -> i32 {
    if matrix.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: 16 readable floats (the contract); no alignment is assumed.
    let matrix = unsafe { ptr::read_unaligned(matrix.cast::<[f32; 16]>()) };
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.host.set_camera(VIEW);
            let version = shell.host.view(VIEW).map_or(0, |v| v.camera);
            shell.host.backend.set_camera_matrix(VIEW, version, matrix);
            Ok(())
        })
    }
}

/// Sets the view's viewport in pixels; it must equal the surface's size, which the shell sets with `sa2_resize`.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6h_set_viewport(handle: *mut ShellHost, width: u32, height: u32) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.host.set_viewport(VIEW, (width, height));
            Ok(())
        })
    }
}

/// Polls the host: delivers the view jobs that ended ([`m6h_run_view`]), notices a lost device, and installs ready
/// preparations.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6h_pump(handle: *mut ShellHost) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.poll();
            Ok(())
        })
    }
}

/// Polls the host as [`m6h_pump`] does, then submits a frame of the view to the surface and writes its sequence,
/// device epoch, surface and revisions to `*out`. A lost device answers [`M6H_E_UNAVAILABLE`]. The frame reports are
/// read and dropped here.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_frame(handle: *mut ShellHost, out: *mut M6hFrame) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.poll();
            let stamp = shell.host.frame(VIEW).map_err(host_error)?;
            let _ = shell.host.backend.read_frame_reports();
            let drawn = match shell.host.installed(VIEW) {
                Some(installed) if stamp.publication_generation != 0 => {
                    let command =
                        if installed.generation == stamp.publication_generation { installed.command.0 } else { 0 };
                    Some((installed.publication.base.state, command))
                }
                _ => None,
            };
            let (publication_revision, command) = drawn.unwrap_or((0, 0));
            let stale = shell.host.stale(VIEW);
            shell.views.submitted(stamp.sequence, command, stale);
            if !out.is_null() {
                out.write(M6hFrame {
                    sequence: stamp.sequence,
                    device_epoch: stamp.device_epoch,
                    surface: stamp.surface,
                    engine_revision: stamp.engine_revision,
                    publication_revision,
                });
            }
            Ok(())
        })
    }
}

/// Writes up to `capacity` confirmed presentations, oldest first, each with its display time, and their number to
/// `*out_count`. Rows that do not fit wait for the next call, so a caller reads again while `*out_count` equals
/// `capacity`. A display time that names another frame than its presentation answers [`M6H_E_HOST`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `rows` points to `capacity` writable rows (or is null
/// with `capacity` 0); `out_count` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_read_presented(
    handle: *mut ShellHost,
    rows: *mut M6hPresented,
    capacity: u32,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() || (rows.is_null() && capacity > 0) {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let frames = shell.host.read_presented();
            let times = shell.host.backend.take_display_times();
            if times.len() != frames.len() {
                return Err((M6H_E_HOST, "the presentations and their display times differ in number".to_owned()));
            }
            let rows_read: Result<Vec<_>, Failure> =
                frames.iter().zip(&times).map(|(frame, time)| M6hPresented::new(frame, time)).collect();
            shell.presented.extend(rows_read?);
            for frame in &frames {
                shell.views.confirmed(frame.stamp.sequence);
            }
            let n = shell.presented.len().min(capacity as usize);
            for (i, row) in shell.presented.drain(..n).enumerate() {
                rows.add(i).write(row);
            }
            *out_count = n as u32;
            Ok(())
        })
    }
}

/// Recovers a removed device (a new device epoch) and starts the view's preparation for the new device. When the
/// installed publication is the newest one the library published, the host prepares it again under its own command
/// (RM.23). Otherwise the newest one was never installed, because it was published or adopted while the device was
/// lost, and the library publishes it again.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6h_recover(handle: *mut ShellHost) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.host.recover().map_err(host_error)?;
            if shell.host.installed(VIEW).is_some_and(|installed| Some(installed.command) == shell.reference) {
                shell.host.pump();
                Ok(())
            } else {
                shell.publish()
            }
        })
    }
}

/// `len` bytes at `text` as UTF-8.
///
/// # Safety
///
/// `text` is null or points to `len` readable bytes that stay valid while the answer is used.
unsafe fn utf8<'a>(text: *const u8, len: usize, what: &str) -> Result<&'a str, Failure> {
    if text.is_null() {
        return Err((M6H_E_INVALID_ARGUMENT, format!("a null {what}")));
    }
    // SAFETY: the contract.
    let bytes = unsafe { std::slice::from_raw_parts(text, len) };
    std::str::from_utf8(bytes).map_err(|_| (M6H_E_INVALID_ARGUMENT, format!("the {what} is not UTF-8")))
}

/// Opens the turns' session (the [`turns`] module) of `puzzle` ([`M6H_PUZZLE_JUMBLE_OFF`] or
/// [`M6H_PUZZLE_JUMBLE_ON`]; another value answers [`M6H_E_INVALID_ARGUMENT`]) in the folder `folder` (UTF-8,
/// `folder_len` bytes), with the twist menu of the record file `menu` (UTF-8, `menu_len` bytes; an object whose `menu`
/// field is the record) pinned.
///
/// - Without a fixture (`fixture` null and `fixture_len` 0) the session is a new journal at the solved state; the
///   folder is created if missing and must hold nothing yet. With Jumble on the session commits `jumble.on` at once,
///   and the host adopts the solved poses at engine revision [`ENGINE_REVISION`] plus one.
/// - With one, `fixture` (UTF-8, `fixture_len` bytes) is the path of a session fixture's record ([`fixture`]) of the
///   same puzzle and the folder a fresh copy of its session folder. The session reopens there; its head must be the
///   fixture's. The host adopts it at the engine revision the fixture's record names and publishes the view for it.
///
/// `*out_revision`, when not null, gets the engine revision the session opened at (charter E3's first correct frame
/// draws it). The adopted state must still be the solved state at [`ENGINE_REVISION`], and a handle opens one session
/// in its life. A session whose adoption the host or the core refuses is closed again (its journal stays in the
/// folder), and the handle keeps the solved state.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `folder`, `menu` and a non-null `fixture` point to their
/// readable bytes; `out_revision` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_open_session(
    handle: *mut ShellHost,
    puzzle: u32,
    folder: *const u8,
    folder_len: usize,
    menu: *const u8,
    menu_len: usize,
    fixture: *const u8,
    fixture_len: usize,
    out_revision: *mut u64,
) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let puzzle = match puzzle {
                M6H_PUZZLE_JUMBLE_OFF => Puzzle::JumbleOff,
                M6H_PUZZLE_JUMBLE_ON => Puzzle::JumbleOn,
                other => return Err((M6H_E_INVALID_ARGUMENT, format!("{other} is not a puzzle of this interface"))),
            };
            let folder = Path::new(utf8(folder, folder_len, "session folder")?);
            let menu = Path::new(utf8(menu, menu_len, "twist menu path")?);
            let fixture = match (fixture.is_null(), fixture_len) {
                (true, 0) => None,
                _ => Some(Path::new(utf8(fixture, fixture_len, "session fixture path")?)),
            };
            if shell.turns.is_some() || !shell.check.is_empty() || shell.revision != ENGINE_REVISION {
                return Err((M6H_E_SESSION, "this handle has opened its session already".to_owned()));
            }
            // The preload's exact model, joined where the open would load it; without one (or on a second attempt
            // after a failed open) the open loads it.
            let (pending, assets) = (shell.exact.take(), shell.assets.clone());
            let exact = move || match pending {
                Some(load) => preload::exact_result(load.join()),
                None => preload::load_exact(&assets),
            };
            let mut turns = match fixture {
                None => Turns::open_with(shell.kernel.clone(), exact, folder, menu, puzzle),
                Some(path) => crate::fixture::read(path).and_then(|fixture| {
                    if fixture.puzzle != puzzle {
                        let names = (fixture.puzzle.name(), puzzle.name());
                        return Err(format!("the session fixture's puzzle is {}, not {}", names.0, names.1));
                    }
                    Turns::reopen_with(shell.kernel.clone(), exact, folder, menu, &fixture)
                }),
            }
            .map_err(|reason| (M6H_E_SESSION, reason))?;
            let (presses, _) = turns.presses();
            let at = puzzle.opened_at() + presses;
            if at > ENGINE_REVISION {
                let head = turns.head().map_err(|e| (M6H_E_SESSION, e))?;
                shell.adopt(&head, at)?;
            }
            shell.turns = Some(turns);
            if !out_revision.is_null() {
                out_revision.write(shell.revision);
            }
            Ok(())
        })
    }
}

/// Starts the language runtime's process (route RT1) from `program` and waits for its hello, as
/// [`m6h_load_runtime`] does.
pub fn start_runtime(program: &Path) -> Result<Runtime, String> {
    let mut runtime = Runtime::new(RuntimeConfig::new(program));
    runtime.start().map_err(|fault| format!("the runtime did not start: {fault}"))?;
    Ok(runtime)
}

/// Starts the language runtime's process (route RT1, c600-runtime's child) from the program `program` (UTF-8,
/// `program_len` bytes; a binary that runs `c600_runtime::child::main`) and waits for its hello, which names the
/// protocol, the IR version and the execution contract. The handle keeps the process until [`m6h_destroy`], which
/// ends it; the view jobs run in it ([`m6h_run_view`]). A program that does not start, and a second load, answer
/// [`M6H_E_RUNTIME`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `program` points to its readable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6h_load_runtime(handle: *mut ShellHost, program: *const u8, program_len: usize) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let program = utf8(program, program_len, "runtime program path")?;
            if shell.runtime.is_some() || shell.views.worker.is_some() {
                return Err((M6H_E_RUNTIME, "this handle has started its runtime already".to_owned()));
            }
            let runtime = start_runtime(Path::new(program)).map_err(|reason| (M6H_E_RUNTIME, reason))?;
            shell.runtime = Some((runtime, PathBuf::from(program)));
            Ok(())
        })
    }
}

/// Schedules a view job (C9 U8, a draft): runs the program text `source` (UTF-8, `source_len` bytes; one `.6cs`
/// module) from its exported function `entry` (UTF-8, `entry_len` bytes; no arguments) in the runtime that
/// [`m6h_load_runtime`] started, and writes the job's ticket, its display-host command id, to `*out_ticket`.
///
/// The call returns at once. It schedules the job on the view with `Host::schedule`, which supersedes a run still
/// pending, and records the adopted engine revision. The job runs on the handle's worker thread, under the limits
/// [`VIEW_LIMITS`] and the deadline [`VIEW_DEADLINE`], with no capability granted and no host row served. When it
/// ends, [`m6h_pump`] or [`m6h_frame`] on the caller's thread delivers, for a completed run, the reference
/// publication of the recorded revision with the plan dependency `program` set to the hex SHA-256 of `source`, under
/// the output allowance [`VIEW_ALLOWANCE`]. A run that the runtime refuses or that faults, a delivery the host
/// refuses, a run that a newer one supersedes, and a run whose revision a twist replaced before its publication was
/// installed end refused; [`m6h_view_result`] reads the record.
///
/// No loaded runtime answers [`M6H_E_RUNTIME`]; a null or non-UTF-8 text [`M6H_E_INVALID_ARGUMENT`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `source` and `entry` point to their readable bytes;
/// `out_ticket` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_run_view(
    handle: *mut ShellHost,
    source: *const u8,
    source_len: usize,
    entry: *const u8,
    entry_len: usize,
    out_ticket: *mut u64,
) -> i32 {
    if out_ticket.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let source = utf8(source, source_len, "program text")?;
            let entry = utf8(entry, entry_len, "entry name")?;
            let ticket = shell.run_view(source, entry)?;
            out_ticket.write(ticket);
            Ok(())
        })
    }
}

/// Writes the record of the view job or draft `ticket` to `*out` ([`M6hViewResult`]). A refused run's code and
/// reason are the `{"code", "reason"}` JSON object that [`m6h_last_error`] answers after this call. The frames in the
/// record come from the confirmed presentations that [`m6h_read_presented`] read. A handle keeps the newest 1024
/// records; an unknown or dropped ticket answers [`M6H_E_INVALID_ARGUMENT`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `out` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_view_result(handle: *mut ShellHost, ticket: u64, out: *mut M6hViewResult) -> i32 {
    if out.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let record = shell
                .views
                .records
                .get(&ticket)
                .ok_or_else(|| (M6H_E_INVALID_ARGUMENT, format!("no view job or draft has ticket {ticket}")))?;
            let row = record.row();
            if row.status == M6H_VIEW_STATUS_REFUSED {
                shell.error = record.refusal.clone();
            }
            out.write(row);
            Ok(())
        })
    }
}

/// Names the editor's program text by its SHA-256 `digest` (32 bytes) without running it (C9 U8, a draft), and writes
/// the draft's ticket to `*out_ticket`. The plan dependency `program` takes the digest, so the view's output reads
/// stale while it came from another text; the draft's record (kind [`M6H_VIEW_KIND_DRAFT`], status
/// [`M6H_VIEW_STATUS_PUBLISHED`] at once) gets the first confirmed frame after the call that drew stale output and
/// the first later one that did not. A draft cancels no run: while a delivered run is being prepared, the dependency
/// keeps that run's digest, and the draft's digest takes its place once the run has ended.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `digest` points to 32 readable bytes; `out_ticket` is
/// writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_view_draft(handle: *mut ShellHost, digest: *const u8, out_ticket: *mut u64) -> i32 {
    if digest.is_null() || out_ticket.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let digest = digest.cast::<[u8; 32]>().read_unaligned();
            let ticket = shell.view_draft(digest);
            out_ticket.write(ticket);
            Ok(())
        })
    }
}

/// Routes one key-down through the session's input router: `chord` (UTF-8, `chord_len` bytes) is c600-input's chord
/// form, a W3C `code` with its modifiers (`Shift+KeyJ`), and `repeat` is nonzero for the platform's auto-repeat. A
/// twist key that commits adopts the new head at the next engine revision and publishes the view for it; `*out`
/// says which press it was and the adopted revision after it. A chord the router does not know answers
/// [`M6H_E_INVALID_ARGUMENT`], and no open session [`M6H_E_SESSION`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `chord` points to its readable bytes; `out` is null or
/// writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_key_down(
    handle: *mut ShellHost,
    chord: *const u8,
    chord_len: usize,
    repeat: u32,
    out: *mut M6hKey,
) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let chord = utf8(chord, chord_len, "chord")?;
            let press = shell.turns()?.key_down(chord, repeat != 0).map_err(|e| (M6H_E_INVALID_ARGUMENT, e))?;
            let code = match press {
                Press::None => M6H_PRESS_NONE,
                Press::Refused => M6H_PRESS_REFUSED,
                Press::Committed | Press::Other => {
                    let head = shell.turns()?.head().map_err(|e| (M6H_E_SESSION, e))?;
                    shell.advance(&head)?;
                    if press == Press::Committed {
                        M6H_PRESS_COMMITTED
                    } else {
                        M6H_PRESS_OTHER
                    }
                }
            };
            if !out.is_null() {
                out.write(M6hKey { press: code, reserved: 0, engine_revision: shell.revision });
            }
            Ok(())
        })
    }
}

/// Routes a key-up of `chord` (as for [`m6h_key_down`]; its modifiers are ignored).
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `chord` points to its readable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6h_key_up(handle: *mut ShellHost, chord: *const u8, chord_len: usize) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let chord = utf8(chord, chord_len, "chord")?;
            shell.turns()?.key_up(chord).map_err(|e| (M6H_E_INVALID_ARGUMENT, e))
        })
    }
}

/// Tells the session's input router that the window lost the keyboard focus, which releases its held keys.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6h_focus_lost(handle: *mut ShellHost) -> i32 {
    // SAFETY: the contract.
    unsafe { with_host(handle, |shell| shell.turns()?.focus_lost().map_err(|e| (M6H_E_SESSION, e))) }
}

/// Closes the session and runs its correctness control once ([`turns::Turns::check`]). `shown_revision` is the engine
/// revision of the newest frame the shell saw presented (0 for none). `*out_pass` gets 1 when the control passed and
/// 0 otherwise; [`m6h_check`] gives its record. Later key calls answer [`M6H_E_SESSION`].
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `out_pass` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_close_session(handle: *mut ShellHost, shown_revision: u64, out_pass: *mut u32) -> i32 {
    if out_pass.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let mut turns = shell.turns.take().ok_or_else(|| (M6H_E_SESSION, "no session is open".to_owned()))?;
            let shown = (shown_revision != 0)
                .then(|| (shown_revision, shell.digests.get(&shown_revision).cloned().unwrap_or_default()));
            let record = turns.check(shown);
            let pass = matches!(record.get("pass"), Some(Value::Bool(true)));
            shell.check = record.canonical();
            out_pass.write(u32::from(pass));
            Ok(())
        })
    }
}

/// Writes `text` with a NUL to `buffer` and its size with the NUL to `*out_size`; a smaller buffer answers
/// [`M6H_E_INVALID_ARGUMENT`] and writes nothing, as the core's JSON calls do.
///
/// # Safety
///
/// `buffer` points to `capacity` writable bytes (or is null); `out_size` is writable.
unsafe fn copy_text(text: &str, buffer: *mut u8, capacity: u32, out_size: *mut u32) -> i32 {
    let needed = text.len() + 1;
    // SAFETY: the contract.
    unsafe {
        *out_size = needed as u32;
        if buffer.is_null() || (capacity as usize) < needed {
            return M6H_E_INVALID_ARGUMENT;
        }
        ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
        *buffer.add(text.len()) = 0;
    }
    M6H_OK
}

/// The correctness control's record that [`m6h_close_session`] made, as NUL-terminated canonical JSON (the run
/// row's `correctness_detail`), sized as [`m6h_last_error`] sizes its text. Before the close the record is empty.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `buffer` points to `capacity` writable bytes (or is
/// null); `out_size` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_check(handle: *mut ShellHost, buffer: *mut u8, capacity: u32, out_size: *mut u32) -> i32 {
    if out_size.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    match unsafe { handle.as_ref() } {
        // SAFETY: the contract.
        Some(shell) => unsafe { copy_text(&shell.check, buffer, capacity, out_size) },
        None => M6H_E_INVALID_ARGUMENT,
    }
}

/// The reason for the handle's last failed call, or with a null handle for this thread's last failed
/// [`m6h_create`], as NUL-terminated UTF-8. `*out_size` gets the size with the NUL; a smaller buffer answers
/// [`M6H_E_INVALID_ARGUMENT`] and writes nothing, as the core's JSON calls do.
///
/// # Safety
///
/// `handle` is null or a live handle that no other thread is using; `buffer` points to `capacity` writable bytes (or
/// is null); `out_size` is writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_last_error(
    handle: *mut ShellHost,
    buffer: *mut u8,
    capacity: u32,
    out_size: *mut u32,
) -> i32 {
    if out_size.is_null() {
        return M6H_E_INVALID_ARGUMENT;
    }
    // SAFETY: the contract.
    let text = match unsafe { handle.as_ref() } {
        Some(shell) => shell.error.clone(),
        None => CREATE_ERROR.with(|e| e.borrow().clone()),
    };
    // SAFETY: the contract.
    unsafe { copy_text(&text, buffer, capacity, out_size) }
}

/// Frees the handle. The core's context stays the shell's to detach. The runtime's process ends, a view job in flight
/// with it, and the call waits for the worker thread.
///
/// # Safety
///
/// `handle` is null or a live handle that no other thread is using; it is not used again.
#[no_mangle]
pub unsafe extern "C" fn m6h_destroy(handle: *mut ShellHost) {
    if !handle.is_null() {
        // SAFETY: the contract; a panic in a destructor is not carried across the C boundary.
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(handle) })));
    }
}
