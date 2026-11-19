//! The native host library of C9's product shell, step S2 of `renderer/shell/README.md` (a draft): c600-display's
//! host over the bridge, behind a C interface that the shell's C# side calls.
//!
//! Route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks: the shell loads
//! `sa2_interop.dll`, creates a context on the core's own device with `sa2_create`, attaches the surface beneath its
//! window, and hands this library the core's function table ([`CoreFunctions`]) and the context. Frames go to the
//! surface (`SA2_TARGET_SURFACE`). This library loads no library and calls no Windows function (CT-RPI-20).
//!
//! Drafts that the sources leave open, each to be replaced by LP3's parts:
//! - One view, which shows the adopted state as one retained publication of every slot, visible and interactive.
//!   There is no picking or preview yet.
//! - The adopted state starts as the shipped model's solved state at engine revision [`ENGINE_REVISION`]. With a
//!   session open ([`m6h_open_session`], charter E2), each twist key the shell routes ([`m6h_key_down`]) that commits
//!   `twist.apply` (the [`turns`] module) adopts the layer's new state at the next engine revision and publishes the
//!   view again for it, so the view's output is never stale (RPI.13) for longer than its preparation takes. A frame
//!   shows a press once its engine revision and its publication's revision have both reached the press's revision.
//! - The host context is c600-display's declared fixture context, with the core's own support report in place of the
//!   declared one. Its tokens, words and stage signatures are fixtures, not the checked library's definitions, and the
//!   limit profile [`LIMITS`] is a declared value, not a measured one.
//! - After a recovery the library publishes the view again, so that it gets resources on the new device (RM.23).
//!   c600-display's `Host::recover` keeps the old resource set, which the bridge refuses after a recovery.
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

use c600_display::array::Array;
use c600_display::codes;
use c600_display::host::{Backend, Host};
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

pub mod turns;
use turns::{Press, Turns};

/// The version of this C interface. The shell refuses a library that answers another. Version 2 added the frame's
/// revisions and the session calls.
pub const M6H_ABI_VERSION: u32 = 2;

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

/// [`M6hKey::press`]: the key-down was no measured press (no twist key, or a repeat).
pub const M6H_PRESS_NONE: u32 = 0;
/// [`M6hKey::press`]: a twist key committed `twist.apply`.
pub const M6H_PRESS_COMMITTED: u32 = 1;
/// [`M6hKey::press`]: a twist key that the router or the layer refused; the state did not change.
pub const M6H_PRESS_REFUSED: u32 = 2;
/// [`M6hKey::press`]: no measured press, but another row changed the state (the draft map's `session.undo` and
/// `session.redo`). The host adopts and shows the new state; the correctness control then fails, since its replay
/// knows only twists.
pub const M6H_PRESS_OTHER: u32 = 3;

/// The one view of the draft.
pub const VIEW: ViewId = ViewId(1);
/// The engine revision of the adopted state.
pub const ENGINE_REVISION: u64 = 1;
/// The declared limit profile of the draft, not a measured one.
pub const LIMITS: &str = "shell-draft/1";

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
    publication: Publication,
    next_command: u64,
    /// The adopted engine revision, and each adopted revision's state digest.
    revision: u64,
    digests: BTreeMap<u64, String>,
    turns: Option<Turns>,
    /// The correctness control's record, once [`m6h_close_session`] made it.
    check: String,
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
fn engine_bundle(state: &Snapshot, revision: &RevisionRef) -> EngineBundle {
    let set = full_set();
    let input = produce::retained(state, &set, &set, Vec::new());
    let arrays = [("labels", input.labels), ("visibility", input.visibility), ("interaction", input.interaction)];
    let bound = |(name, array): (&str, Array)| BoundArray { name: name.to_owned(), revision: revision.state, array };
    EngineBundle { revision: revision.clone(), arrays: arrays.into_iter().map(bound).collect() }
}

/// The draft's publication of [`VIEW`]: one plan with one retained primitive of every slot, and no instances.
fn publication(geometry: &GeometryInput, state: &Snapshot, base: &RevisionRef) -> Publication {
    let set = full_set();
    let plan = DisplayPlan {
        plan: PlanId(1),
        base: base.clone(),
        subject: StateRef::Snapshot(base.clone()),
        derivation: None,
        reachability: None,
        dimension: 4,
        geometry: GeometryBinding { identity: geometry.identity.clone(), digests: geometry.digests() },
        primitives: vec![Primitive::Retained(produce::retained(state, &set, &set, Vec::new()))],
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
    fn create(core: DllCore, assets: &Path) -> Result<ShellHost, Failure> {
        let kernel = Arc::new(Kernel::load(assets).map_err(model_error)?);
        let geometry = GeometryInput::load(assets).map_err(model_error)?;
        let state = kernel.solved();
        let base = produce::revision(&geometry.identity, ENGINE_REVISION, &state);
        let limits = LimitProfile::uniform(LIMITS, 1 << 40);
        let mut backend = RenderBackend::new(core, limits.clone());
        backend.adopt_engine(&engine_bundle(&state, &base)).map_err(host_error)?;
        let support = backend.describe_backend().map_err(host_error)?;
        backend.set_frame_target(TARGET_SURFACE);
        backend.set_camera_matrix(VIEW, 0, IDENTITY);
        let mut ctx = produce::fixture_context(&geometry, limits);
        ctx.support = support;
        let mut host = Host::new(backend, ctx, base.clone());
        host.open_view(VIEW).map_err(host_error)?;
        let mut shell = ShellHost {
            host,
            publication: publication(&geometry, &state, &base),
            kernel,
            geometry,
            assets: assets.to_path_buf(),
            next_command: 1,
            revision: ENGINE_REVISION,
            digests: BTreeMap::from([(ENGINE_REVISION, state.digest().to_owned())]),
            turns: None,
            check: String::new(),
            presented: VecDeque::new(),
            error: String::new(),
            poisoned: false,
        };
        shell.publish()?;
        Ok(shell)
    }

    /// Adopts `state` at the next engine revision, in the core first and then in the host, and publishes the view
    /// for it. While the device is lost the publication waits for [`m6h_recover`], which publishes it.
    fn advance(&mut self, state: &Snapshot) -> Result<u64, Failure> {
        let revision = self.revision + 1;
        let base = produce::revision(&self.geometry.identity, revision, state);
        let bundle = engine_bundle(state, &base);
        self.host.backend.adopt_engine(&bundle).map_err(host_error)?;
        self.host.adopt(&bundle).map_err(host_error)?;
        self.revision = revision;
        self.digests.insert(revision, state.digest().to_owned());
        self.publication = publication(&self.geometry, state, &base);
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
        let receipt = self.host.publish(command, self.publication.clone());
        match receipt.status {
            CommandStatus::Refused(error) | CommandStatus::Failed(error) => Err(host_error(error)),
            _ => {
                self.host.pump();
                Ok(())
            }
        }
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
        ShellHost::create(core, Path::new(path))
    }));
    let failure = match attempt {
        Ok(Ok(shell)) => {
            // SAFETY: as above.
            unsafe { *out = Box::into_raw(Box::new(shell)) };
            return M6H_OK;
        }
        Ok(Err(failure)) => failure,
        Err(_) => (M6H_E_POISONED, "creation panicked".to_owned()),
    };
    CREATE_ERROR.with(|e| *e.borrow_mut() = failure.1);
    failure.0
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

/// Polls the host: notices a lost device, and installs ready preparations.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using.
#[no_mangle]
pub unsafe extern "C" fn m6h_pump(handle: *mut ShellHost) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.host.pump();
            Ok(())
        })
    }
}

/// Polls the host, then submits a frame of the view to the surface and writes its sequence, device epoch, surface and
/// revisions to `*out`. A lost device answers [`M6H_E_UNAVAILABLE`]. The frame reports are read and dropped here.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn m6h_frame(handle: *mut ShellHost, out: *mut M6hFrame) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            shell.host.pump();
            let stamp = shell.host.frame(VIEW).map_err(host_error)?;
            let _ = shell.host.backend.read_frame_reports();
            let publication_revision = match shell.host.installed(VIEW) {
                Some(installed) if stamp.publication_generation != 0 => installed.publication.base.state,
                _ => 0,
            };
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
            let n = shell.presented.len().min(capacity as usize);
            for (i, row) in shell.presented.drain(..n).enumerate() {
                rows.add(i).write(row);
            }
            *out_count = n as u32;
            Ok(())
        })
    }
}

/// Recovers a removed device (a new device epoch) and publishes the view again for the new device.
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
            shell.publish()
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

/// Opens the turns' session (the [`turns`] module): a new journal in the folder `folder` (UTF-8, `folder_len` bytes;
/// created if missing, and it must hold nothing yet), at the solved state, with the twist menu of the record file
/// `menu` (UTF-8, `menu_len` bytes; an object whose `menu` field is the record) pinned. The adopted state must still
/// be the solved state at [`ENGINE_REVISION`], and a handle opens one session in its life.
///
/// # Safety
///
/// `handle` is a live handle that no other thread is using; `folder` and `menu` point to their readable bytes.
#[no_mangle]
pub unsafe extern "C" fn m6h_open_session(
    handle: *mut ShellHost,
    folder: *const u8,
    folder_len: usize,
    menu: *const u8,
    menu_len: usize,
) -> i32 {
    // SAFETY: the contract.
    unsafe {
        with_host(handle, |shell| {
            let folder = utf8(folder, folder_len, "session folder")?;
            let menu = utf8(menu, menu_len, "twist menu path")?;
            if shell.turns.is_some() || !shell.check.is_empty() || shell.revision != ENGINE_REVISION {
                return Err((M6H_E_SESSION, "this handle has opened its session already".to_owned()));
            }
            let turns = Turns::open(shell.kernel.clone(), &shell.assets, Path::new(folder), Path::new(menu))
                .map_err(|reason| (M6H_E_SESSION, reason))?;
            shell.turns = Some(turns);
            Ok(())
        })
    }
}

/// Routes one key-down through the session's input router: `chord` (UTF-8, `chord_len` bytes) is c600-input's chord
/// form, a W3C `code` with its modifiers (`Shift+KeyJ`), and `repeat` is nonzero for the platform's auto-repeat. A
/// twist key that commits adopts the new state at the next engine revision and publishes the view for it; `*out`
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
                    let state = shell.turns()?.state().map_err(|e| (M6H_E_SESSION, e))?;
                    shell.advance(&state)?;
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

/// Frees the handle. The core's context stays the shell's to detach.
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
