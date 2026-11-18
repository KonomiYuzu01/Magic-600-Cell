//! A C shim over the fake core: `extern "C"` functions with the header's signatures that check what a caller passes
//! (struct sizes, array lengths, buffer sizes) and forward to a [`FakeCore`] behind the context pointer. With it, the
//! bridge runs through [`DllCore`] and the function table exactly as it would over `sa2_interop.dll`.

#![allow(dead_code)]

use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::slice;

use c600_render::abi::*;
use c600_render::{CoreAbi, CoreFunctions, DllCore, EngineImage, FakeCore, PoseArrays, PreparationInput};

/// What a context points to.
pub struct Shim {
    pub fake: FakeCore,
    /// Leading spaces before the JSON `sa2_describe` writes, to make a report longer than the first buffer.
    pub pad: usize,
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
    4
}

pub unsafe extern "C" fn abi_version_3() -> u32 {
    3
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
    code(unsafe { shim(ctx) }.fake.poll(ticket), |row| unsafe { *out = row })
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
        let dll = unsafe { DllCore::new(functions(), shim.cast()) }.expect("the shim speaks ABI 4");
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
}

/// A core whose fake a test can drive (complete work, place picks, remove the device).
pub trait Driver: CoreAbi {
    fn fake(&mut self) -> &mut FakeCore;
}

impl Driver for FakeCore {
    fn fake(&mut self) -> &mut FakeCore {
        self
    }
}

impl Driver for Shimmed {
    fn fake(&mut self) -> &mut FakeCore {
        // SAFETY: the shim lives as long as `self`, and the borrow of `self` keeps the DLL side from running.
        unsafe { &mut (*self.shim).fake }
    }
}
