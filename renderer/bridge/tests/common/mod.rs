//! A C shim over the fake core: `extern "C"` functions with the header's signatures that check what a caller passes
//! (struct sizes, array lengths, buffer sizes) and forward to a [`FakeCore`] behind the context pointer. With it, the
//! bridge runs through [`DllCore`] and the function table exactly as it would over `sa2_interop.dll`.

#![allow(dead_code)]

use std::cell::Cell;
use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::slice;

use c600_render::abi::*;
use c600_render::{CoreAbi, EngineImage, FakeCore, PoseArrays, PreparationInput};
use c600_render_ffi::{CoreFunctions, DllCore};

/// c600-render's host scenarios and their [`Driver`] trait, from that crate's tests, so that the same scenarios run
/// here through the C function table.
#[path = "../../../../rust/c600-render/tests/support/mod.rs"]
pub mod support;

pub use support::Driver;

/// What a context points to.
pub struct Shim {
    pub fake: FakeCore,
    /// Leading spaces before the JSON `sa2_describe` writes, to make a report longer than the first buffer.
    pub pad: usize,
}

thread_local! {
    /// The `sa2_poll` calls left on this thread until the fake core's GPU reaches every fence ([`complete_after_polls`]).
    static POLLS_LEFT: Cell<Option<u32>> = const { Cell::new(None) };
}

/// From now on, on this thread, the `polls`-th `sa2_poll` first has the fake core's GPU reach every fence
/// ([`FakeCore::complete`]): a preparation pending now answers pending to the `polls - 1` polls before it, as an upload
/// behind the GPU's current work does. Once, and per thread, so that tests running in parallel do not share it.
pub fn complete_after_polls(polls: u32) {
    POLLS_LEFT.with(|left| left.set(Some(polls.max(1))));
}

unsafe fn shim<'a>(ctx: *mut c_void) -> &'a mut Shim {
    unsafe { &mut *ctx.cast::<Shim>() }
}

fn code<T>(result: Result<T, Status>, write: impl FnOnce(T)) -> i32 {
    match result {
        Ok(value) => {
            write(value);
            Status::OK.0
        }
        Err(status) => status.0,
    }
}

/// The header's JSON rule: `out_size` gets the size with the NUL; a smaller buffer is `SA2_E_INVALID_ARGUMENT`.
unsafe fn write_json(status: Status, text: &str, buffer: *mut c_char, size: u32, out_size: *mut u32) -> i32 {
    let needed = text.len() + 1;
    unsafe { *out_size = needed as u32 };
    if buffer.is_null() || (size as usize) < needed {
        return Status::INVALID_ARGUMENT.0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), buffer.cast::<u8>(), text.len());
        *buffer.add(text.len()) = 0;
    }
    status.0
}

fn sized<T>(struct_size: u32) -> bool {
    struct_size as usize == size_of::<T>()
}

unsafe extern "C" fn abi_version() -> u32 {
    ABI_VERSION
}

/// The previous ABI's answer, which `DllCore` refuses.
pub unsafe extern "C" fn abi_version_previous() -> u32 {
    ABI_VERSION - 1
}

unsafe extern "C" fn last_error(ctx: *mut c_void, buffer: *mut c_char, size: u32) -> i32 {
    let text = unsafe { shim(ctx) }.fake.last_error();
    let n = text.len().min(size.saturating_sub(1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), buffer.cast::<u8>(), n);
        *buffer.add(n) = 0;
    }
    Status::OK.0
}

unsafe extern "C" fn describe(ctx: *mut c_void, buffer: *mut c_char, size: u32, out_size: *mut u32) -> i32 {
    let shim = unsafe { shim(ctx) };
    match shim.fake.describe() {
        Ok(json) => unsafe {
            write_json(Status::OK, &format!("{}{json}", " ".repeat(shim.pad)), buffer, size, out_size)
        },
        Err(status) => status.0,
    }
}

unsafe extern "C" fn engine_adopt(ctx: *mut c_void, image: *const Sa2EngineImage, out: *mut u64) -> i32 {
    let image = unsafe { &*image };
    if !sized::<Sa2EngineImage>(image.struct_size) || image.labels.is_null() {
        return Status::INVALID_ARGUMENT.0;
    }
    let n = image.pose_count as usize;
    let pose = if image.pose_index.is_null() {
        None
    } else {
        Some(unsafe {
            PoseArrays {
                pose_index: slice::from_raw_parts(image.pose_index, PIECES),
                poses: slice::from_raw_parts(image.poses, n * 16),
                pose_lattice: slice::from_raw_parts(image.pose_lattice, n),
            }
        })
    };
    let view = EngineImage {
        revision: image.revision,
        labels: unsafe { slice::from_raw_parts(image.labels, SLOTS) },
        pose,
        digest: image.digest,
    };
    code(unsafe { shim(ctx) }.fake.engine_adopt(&view), |revision| unsafe { *out = revision })
}

unsafe extern "C" fn prepare(ctx: *mut c_void, row: *const Sa2Preparation, out: *mut u64) -> i32 {
    let row = unsafe { &*row };
    if !sized::<Sa2Preparation>(row.struct_size) || row.reserved != 0 {
        return Status::INVALID_ARGUMENT.0;
    }
    let input = unsafe {
        PreparationInput {
            command: row.command,
            view: row.view,
            view_generation: row.view_generation,
            publication_generation: row.publication_generation,
            device_epoch: row.device_epoch,
            base_revision: row.base_revision,
            visibility: slice::from_raw_parts(row.visibility, SLOTS),
            interaction: slice::from_raw_parts(row.interaction, SLOTS),
            instances: slice::from_raw_parts(row.instances, SLOTS),
            input_digest: row.input_digest,
        }
    };
    code(unsafe { shim(ctx) }.fake.prepare(&input), |ticket| unsafe { *out = ticket })
}

unsafe extern "C" fn poll(ctx: *mut c_void, ticket: u64, out: *mut Sa2PollRow) -> i32 {
    if !sized::<Sa2PollRow>(unsafe { (*out).struct_size }) {
        return Status::INVALID_ARGUMENT.0;
    }
    let shim = unsafe { shim(ctx) };
    POLLS_LEFT.with(|left| match left.get() {
        Some(n) if n <= 1 => {
            left.set(None);
            shim.fake.complete();
        }
        Some(n) => left.set(Some(n - 1)),
        None => {}
    });
    code(shim.fake.poll(ticket), |row| unsafe { *out = row })
}

unsafe extern "C" fn activate(ctx: *mut c_void, set: u64, digest: *const u8) -> i32 {
    let digest: [u8; 32] = unsafe { slice::from_raw_parts(digest, 32) }.try_into().expect("32 bytes");
    code(unsafe { shim(ctx) }.fake.activate(set, &digest), |()| ())
}

unsafe extern "C" fn cancel(ctx: *mut c_void, ticket: u64, reason: i32, by: u64) -> i32 {
    code(unsafe { shim(ctx) }.fake.cancel(ticket, reason, by), |()| ())
}

unsafe extern "C" fn retire(ctx: *mut c_void, set: u64) -> i32 {
    code(unsafe { shim(ctx) }.fake.retire(set), |()| ())
}

unsafe extern "C" fn frame_submit(ctx: *mut c_void, input: *const Sa2FrameInput, out: *mut u64) -> i32 {
    code(unsafe { shim(ctx) }.fake.frame_submit(unsafe { &*input }), |frame| unsafe { *out = frame })
}

unsafe extern "C" fn frame_reports(ctx: *mut c_void, rows: *mut Sa2FrameReport, capacity: u32, out: *mut u32) -> i32 {
    let rows = unsafe { slice::from_raw_parts_mut(rows, capacity as usize) };
    if rows.iter().any(|row| !sized::<Sa2FrameReport>(row.struct_size)) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.frame_reports(rows), |n| unsafe { *out = n as u32 })
}

unsafe extern "C" fn presented(ctx: *mut c_void, rows: *mut Sa2PresentedRow, capacity: u32, out: *mut u32) -> i32 {
    let rows = unsafe { slice::from_raw_parts_mut(rows, capacity as usize) };
    if rows.iter().any(|row| !sized::<Sa2PresentedRow>(row.struct_size)) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.presented(rows), |n| unsafe { *out = n as u32 })
}

unsafe extern "C" fn pick_request(ctx: *mut c_void, frame: u64, x: u32, y: u32, out: *mut u64) -> i32 {
    code(unsafe { shim(ctx) }.fake.pick_request(frame, x, y), |ticket| unsafe { *out = ticket })
}

unsafe extern "C" fn pick_poll(ctx: *mut c_void, ticket: u64, out: *mut Sa2PickRow) -> i32 {
    if !sized::<Sa2PickRow>(unsafe { (*out).struct_size }) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.pick_poll(ticket), |row| unsafe { *out = row })
}

unsafe extern "C" fn recover(ctx: *mut c_void, buffer: *mut c_char, size: u32, out_size: *mut u32) -> i32 {
    let (status, json) = unsafe { shim(ctx) }.fake.recover();
    unsafe { write_json(status, &json, buffer, size, out_size) }
}

unsafe extern "C" fn counters(ctx: *mut c_void, out: *mut Sa2CounterValues) -> i32 {
    if !sized::<Sa2CounterValues>(unsafe { (*out).struct_size }) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.counters(), |row| unsafe { *out = row })
}

unsafe extern "C" fn device_status(ctx: *mut c_void, out: *mut i32) -> i32 {
    let (status, reason) = unsafe { shim(ctx) }.fake.device_status();
    unsafe { *out = reason };
    status.0
}

unsafe extern "C" fn view_shape_set(ctx: *mut c_void, view: u64, shape: *const Sa2ViewShape) -> i32 {
    let shape = unsafe { shape.as_ref() };
    if shape.is_some_and(|s| !sized::<Sa2ViewShape>(s.struct_size)) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.view_shape_set(view, shape), |()| ())
}

unsafe extern "C" fn view_focus_set(ctx: *mut c_void, view: u64, focus: *const Sa2ViewFocus) -> i32 {
    let focus = unsafe { focus.as_ref() };
    if focus.is_some_and(|f| !sized::<Sa2ViewFocus>(f.struct_size)) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.view_focus_set(view, focus), |()| ())
}

unsafe extern "C" fn style_set(ctx: *mut c_void, revision: u64, bytes: *const u8, channel: u32) -> i32 {
    let bytes = (!bytes.is_null()).then(|| unsafe { std::slice::from_raw_parts(bytes, PIECES) });
    code(unsafe { shim(ctx) }.fake.style_set(revision, bytes, channel), |()| ())
}

unsafe extern "C" fn view_style_set(ctx: *mut c_void, slot: u32, style: *const Sa2ViewStyle) -> i32 {
    let style = unsafe { style.as_ref() };
    if style.is_some_and(|s| !sized::<Sa2ViewStyle>(s.struct_size)) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.view_style_set(slot, style), |()| ())
}

unsafe extern "C" fn frame_submit_panes(
    ctx: *mut c_void,
    input: *const Sa2FrameInput,
    panes: *const Sa2Pane,
    count: u32,
    out: *mut u64,
) -> i32 {
    if panes.is_null() || count == 0 || count > PANES_MAX {
        return Status::INVALID_ARGUMENT.0;
    }
    let panes = unsafe { slice::from_raw_parts(panes, count as usize) };
    code(unsafe { shim(ctx) }.fake.frame_submit_panes(unsafe { &*input }, panes), |frame| unsafe { *out = frame })
}

unsafe extern "C" fn frame_panes(
    ctx: *mut c_void,
    frame: u64,
    rows: *mut Sa2PaneRow,
    capacity: u32,
    out: *mut u32,
) -> i32 {
    let rows = unsafe { slice::from_raw_parts_mut(rows, capacity as usize) };
    if rows.iter().any(|row| !sized::<Sa2PaneRow>(row.struct_size)) {
        return Status::INVALID_ARGUMENT.0;
    }
    code(unsafe { shim(ctx) }.fake.frame_panes(frame, rows), |n| unsafe { *out = n as u32 })
}

/// The shim's function table.
pub fn functions() -> CoreFunctions {
    CoreFunctions {
        abi_version,
        last_error,
        describe,
        engine_adopt,
        prepare,
        poll,
        activate,
        cancel,
        retire,
        frame_submit,
        frame_reports,
        presented,
        pick_request,
        pick_poll,
        recover,
        counters,
        device_status,
        view_shape_set,
        style_set,
        view_style_set,
        frame_submit_panes,
        frame_panes,
        view_focus_set,
    }
}

/// A [`DllCore`] over the shim, owning the shim.
pub struct Shimmed {
    pub dll: DllCore,
    shim: *mut Shim,
}

impl Shimmed {
    pub fn new(fake: FakeCore, pad: usize) -> Shimmed {
        let shim = Box::into_raw(Box::new(Shim { fake, pad }));
        // SAFETY: the table is the shim's and the context a live shim, which this value owns and frees last.
        let dll = unsafe { DllCore::new(functions(), shim.cast()) }.expect("the shim speaks ABI 9");
        Shimmed { dll, shim }
    }
}

impl Drop for Shimmed {
    fn drop(&mut self) {
        // SAFETY: `shim` came from Box::into_raw in `new` and nothing uses it after this.
        drop(unsafe { Box::from_raw(self.shim) });
    }
}

impl CoreAbi for Shimmed {
    fn describe(&self) -> Result<String, Status> {
        self.dll.describe()
    }
    fn engine_adopt(&mut self, image: &EngineImage<'_>) -> Result<u64, Status> {
        self.dll.engine_adopt(image)
    }
    fn prepare(&mut self, input: &PreparationInput<'_>) -> Result<u64, Status> {
        self.dll.prepare(input)
    }
    fn poll(&mut self, ticket: u64) -> Result<Sa2PollRow, Status> {
        self.dll.poll(ticket)
    }
    fn activate(&mut self, resource_set: u64, stamp_digest: &[u8; 32]) -> Result<(), Status> {
        self.dll.activate(resource_set, stamp_digest)
    }
    fn cancel(&mut self, ticket: u64, reason: i32, superseded_by: u64) -> Result<(), Status> {
        self.dll.cancel(ticket, reason, superseded_by)
    }
    fn retire(&mut self, resource_set: u64) -> Result<(), Status> {
        self.dll.retire(resource_set)
    }
    fn frame_submit(&mut self, input: &Sa2FrameInput) -> Result<u64, Status> {
        self.dll.frame_submit(input)
    }
    fn frame_reports(&mut self, rows: &mut [Sa2FrameReport]) -> Result<usize, Status> {
        self.dll.frame_reports(rows)
    }
    fn presented(&mut self, rows: &mut [Sa2PresentedRow]) -> Result<usize, Status> {
        self.dll.presented(rows)
    }
    fn pick_request(&mut self, frame_id: u64, x: u32, y: u32) -> Result<u64, Status> {
        self.dll.pick_request(frame_id, x, y)
    }
    fn pick_poll(&mut self, ticket: u64) -> Result<Sa2PickRow, Status> {
        self.dll.pick_poll(ticket)
    }
    fn recover(&mut self) -> (Status, String) {
        self.dll.recover()
    }
    fn counters(&self) -> Result<Sa2CounterValues, Status> {
        self.dll.counters()
    }
    fn device_status(&self) -> (Status, i32) {
        self.dll.device_status()
    }
    fn last_error(&self) -> String {
        self.dll.last_error()
    }
    fn view_shape_set(&mut self, view: u64, shape: Option<&Sa2ViewShape>) -> Result<(), Status> {
        self.dll.view_shape_set(view, shape)
    }
    fn style_set(&mut self, revision: u64, piece_style: Option<&[u8]>, channel: u32) -> Result<(), Status> {
        self.dll.style_set(revision, piece_style, channel)
    }
    fn view_style_set(&mut self, slot: u32, style: Option<&Sa2ViewStyle>) -> Result<(), Status> {
        self.dll.view_style_set(slot, style)
    }
    fn frame_submit_panes(&mut self, input: &Sa2FrameInput, panes: &[Sa2Pane]) -> Result<u64, Status> {
        self.dll.frame_submit_panes(input, panes)
    }
    fn frame_panes(&mut self, frame_id: u64, rows: &mut [Sa2PaneRow]) -> Result<usize, Status> {
        self.dll.frame_panes(frame_id, rows)
    }
    fn view_focus_set(&mut self, view: u64, focus: Option<&Sa2ViewFocus>) -> Result<(), Status> {
        self.dll.view_focus_set(view, focus)
    }
}

impl Driver for Shimmed {
    fn fake(&mut self) -> &mut FakeCore {
        // SAFETY: the shim lives as long as `self`, and the borrow of `self` keeps the DLL side from running.
        unsafe { &mut (*self.shim).fake }
    }
}
