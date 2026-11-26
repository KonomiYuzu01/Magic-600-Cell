//! The foreign half of C9's renderer bridge (`c600-render-ffi`): the S-A2 core's C functions behind c600-render's
//! [`CoreAbi`]. It lives outside the language workspace (`rust/`), because CT-RPI-01 keeps every member of that
//! workspace free of unsafe code and foreign declarations; c600-render itself forbids unsafe code.
//!
//! The crate never loads a library: whoever loads `sa2_interop.dll` and attaches a context (the product shell, or the
//! platform layer of RPI.52) hands over [`CoreFunctions`], filled with the exports named in [`SYMBOLS`], and the
//! context. Every slice length is checked before a call, because the core reads `SA2_SLOTS`, `SA2_PIECES` and
//! `pose_count` entries through the pointers it receives.

use std::ffi::{c_char, c_void};
use std::mem::size_of;

use c600_render::abi::*;
use c600_render::api::{CoreAbi, EngineImage, PreparationInput};

/// The exports [`CoreFunctions`] holds, in its field order.
pub const SYMBOLS: [&str; 20] = [
    "sa2_abi_version",
    "sa2_last_error",
    "sa2_describe",
    "sa2_engine_adopt",
    "sa2_prepare",
    "sa2_poll",
    "sa2_activate",
    "sa2_cancel",
    "sa2_retire",
    "sa2_frame_submit",
    "sa2_frame_reports",
    "sa2_presented",
    "sa2_pick_request",
    "sa2_pick_poll",
    "sa2_recover",
    "sa2_counters",
    "sa2_device_status",
    "sa2_view_shape_set",
    "sa2_style_set",
    "sa2_view_style_set",
];

/// The ABI 8 product functions of one loaded core, in [`SYMBOLS`] order. `#[repr(C)]` so that a host in another
/// language can fill it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CoreFunctions {
    pub abi_version: unsafe extern "C" fn() -> u32,
    pub last_error: unsafe extern "C" fn(*mut c_void, *mut c_char, u32) -> i32,
    pub describe: unsafe extern "C" fn(*mut c_void, *mut c_char, u32, *mut u32) -> i32,
    pub engine_adopt: unsafe extern "C" fn(*mut c_void, *const Sa2EngineImage, *mut u64) -> i32,
    pub prepare: unsafe extern "C" fn(*mut c_void, *const Sa2Preparation, *mut u64) -> i32,
    pub poll: unsafe extern "C" fn(*mut c_void, u64, *mut Sa2PollRow) -> i32,
    pub activate: unsafe extern "C" fn(*mut c_void, u64, *const u8) -> i32,
    pub cancel: unsafe extern "C" fn(*mut c_void, u64, i32, u64) -> i32,
    pub retire: unsafe extern "C" fn(*mut c_void, u64) -> i32,
    pub frame_submit: unsafe extern "C" fn(*mut c_void, *const Sa2FrameInput, *mut u64) -> i32,
    pub frame_reports: unsafe extern "C" fn(*mut c_void, *mut Sa2FrameReport, u32, *mut u32) -> i32,
    pub presented: unsafe extern "C" fn(*mut c_void, *mut Sa2PresentedRow, u32, *mut u32) -> i32,
    pub pick_request: unsafe extern "C" fn(*mut c_void, u64, u32, u32, *mut u64) -> i32,
    pub pick_poll: unsafe extern "C" fn(*mut c_void, u64, *mut Sa2PickRow) -> i32,
    pub recover: unsafe extern "C" fn(*mut c_void, *mut c_char, u32, *mut u32) -> i32,
    pub counters: unsafe extern "C" fn(*mut c_void, *mut Sa2CounterValues) -> i32,
    pub device_status: unsafe extern "C" fn(*mut c_void, *mut i32) -> i32,
    /// ABI 6: a view's shape.
    pub view_shape_set: unsafe extern "C" fn(*mut c_void, u64, *const Sa2ViewShape) -> i32,
    /// ABI 7: one channel of the adopted revision's piece style.
    pub style_set: unsafe extern "C" fn(*mut c_void, u64, *const u8, u32) -> i32,
    /// ABI 7: a style slot.
    pub view_style_set: unsafe extern "C" fn(*mut c_void, u32, *const Sa2ViewStyle) -> i32,
}

/// One attached core context. It holds a raw pointer, so it is neither `Send` nor `Sync`: it stays on the thread
/// that created it, as the header's threading rule requires.
pub struct DllCore {
    functions: CoreFunctions,
    context: *mut c_void,
}

/// The first JSON buffer; a longer report is read again at the size the core names.
const JSON_BUFFER: usize = 4096;
/// The largest JSON report the bridge reads again; a core that names more is answered with the first buffer's text.
pub const JSON_LIMIT: usize = 16 << 20;

impl DllCore {
    /// Binds an attached context. Fails with `SA2_E_INVALID_ARGUMENT` for a null context and `SA2_E_UNSUPPORTED`
    /// when the library is not ABI 8.
    ///
    /// # Safety
    ///
    /// `functions` are the [`SYMBOLS`] exports of one loaded `sa2_interop.dll`; `context` is a context that DLL
    /// attached; the library stays loaded and the context attached, and is used by nothing else, for the life of the
    /// returned value; and the value is used only on the context's thread.
    pub unsafe fn new(functions: CoreFunctions, context: *mut c_void) -> Result<DllCore, Status> {
        if context.is_null() {
            return Err(Status::INVALID_ARGUMENT);
        }
        if unsafe { (functions.abi_version)() } != ABI_VERSION {
            return Err(Status::UNSUPPORTED);
        }
        Ok(DllCore { functions, context })
    }

    /// A JSON-writing call: `call(context, buffer, size, out_size)`, retried once at the size the core names.
    fn json(&self, call: unsafe extern "C" fn(*mut c_void, *mut c_char, u32, *mut u32) -> i32) -> (Status, String) {
        let mut size = JSON_BUFFER;
        for _ in 0..2 {
            let mut buffer = vec![0u8; size];
            let mut needed = 0u32;
            // SAFETY: the buffer holds `size` bytes and `needed` is a live u32 (DllCore::new's contract).
            let status = Status(unsafe { call(self.context, buffer.as_mut_ptr().cast(), size as u32, &mut needed) });
            if status == Status::INVALID_ARGUMENT && needed as usize > size && needed as usize <= JSON_LIMIT {
                size = needed as usize;
                continue;
            }
            return (status, text(&buffer));
        }
        (Status::INVALID_ARGUMENT, String::new())
    }
}

/// The NUL-terminated text at the start of `buffer`.
fn text(buffer: &[u8]) -> String {
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    String::from_utf8_lossy(&buffer[..end]).into_owned()
}

fn capacity(len: usize) -> u32 {
    len.min(u32::MAX as usize) as u32
}

impl CoreAbi for DllCore {
    fn describe(&self) -> Result<String, Status> {
        let (status, json) = self.json(self.functions.describe);
        status.check().map(|()| json)
    }

    fn engine_adopt(&mut self, image: &EngineImage<'_>) -> Result<u64, Status> {
        if image.labels.len() != SLOTS {
            return Err(Status::INVALID_ARGUMENT);
        }
        let (pose_count, pose_index, poses, pose_lattice) = match image.pose {
            None => (0, std::ptr::null(), std::ptr::null(), std::ptr::null()),
            Some(pose) => {
                let n = pose.pose_lattice.len();
                if pose.pose_index.len() != PIECES || n == 0 || n > MAX_POSES || pose.poses.len() != n * 16 {
                    return Err(Status::INVALID_ARGUMENT);
                }
                (n as u32, pose.pose_index.as_ptr(), pose.poses.as_ptr(), pose.pose_lattice.as_ptr())
            }
        };
        let row = Sa2EngineImage {
            struct_size: size_of::<Sa2EngineImage>() as u32,
            pose_count,
            revision: image.revision,
            labels: image.labels.as_ptr(),
            pose_index,
            poses,
            pose_lattice,
            digest: image.digest,
        };
        let mut revision = 0u64;
        // SAFETY: every array the row points to has the length the header requires, checked above.
        Status(unsafe { (self.functions.engine_adopt)(self.context, &row, &mut revision) }).check()?;
        Ok(revision)
    }

    fn prepare(&mut self, input: &PreparationInput<'_>) -> Result<u64, Status> {
        if input.visibility.len() != SLOTS || input.interaction.len() != SLOTS || input.instances.len() != SLOTS {
            return Err(Status::INVALID_ARGUMENT);
        }
        let row = Sa2Preparation {
            struct_size: size_of::<Sa2Preparation>() as u32,
            reserved: 0,
            command: input.command,
            view: input.view,
            view_generation: input.view_generation,
            publication_generation: input.publication_generation,
            device_epoch: input.device_epoch,
            base_revision: input.base_revision,
            visibility: input.visibility.as_ptr(),
            interaction: input.interaction.as_ptr(),
            instances: input.instances.as_ptr(),
            input_digest: input.input_digest,
        };
        let mut ticket = 0u64;
        // SAFETY: the three arrays hold SLOTS entries, checked above.
        Status(unsafe { (self.functions.prepare)(self.context, &row, &mut ticket) }).check()?;
        Ok(ticket)
    }

    fn poll(&mut self, ticket: u64) -> Result<Sa2PollRow, Status> {
        let mut row = Sa2PollRow::new();
        // SAFETY: `row` is a live, sized row.
        Status(unsafe { (self.functions.poll)(self.context, ticket, &mut row) }).check()?;
        Ok(row)
    }

    fn activate(&mut self, resource_set: u64, stamp_digest: &[u8; 32]) -> Result<(), Status> {
        // SAFETY: the digest holds the 32 bytes the core reads.
        Status(unsafe { (self.functions.activate)(self.context, resource_set, stamp_digest.as_ptr()) }).check()
    }

    fn cancel(&mut self, ticket: u64, reason: i32, superseded_by: u64) -> Result<(), Status> {
        // SAFETY: scalar arguments only.
        Status(unsafe { (self.functions.cancel)(self.context, ticket, reason, superseded_by) }).check()
    }

    fn retire(&mut self, resource_set: u64) -> Result<(), Status> {
        // SAFETY: scalar arguments only.
        Status(unsafe { (self.functions.retire)(self.context, resource_set) }).check()
    }

    fn frame_submit(&mut self, input: &Sa2FrameInput) -> Result<u64, Status> {
        let mut row = *input;
        row.struct_size = size_of::<Sa2FrameInput>() as u32;
        let mut frame = 0u64;
        // SAFETY: `row` is a live, sized row.
        Status(unsafe { (self.functions.frame_submit)(self.context, &row, &mut frame) }).check()?;
        Ok(frame)
    }

    fn frame_reports(&mut self, rows: &mut [Sa2FrameReport]) -> Result<usize, Status> {
        rows.iter_mut().for_each(|row| row.struct_size = size_of::<Sa2FrameReport>() as u32);
        let n = capacity(rows.len());
        let mut count = 0u32;
        // SAFETY: `rows` holds `n` sized rows; the core writes at most `n`.
        Status(unsafe { (self.functions.frame_reports)(self.context, rows.as_mut_ptr(), n, &mut count) }).check()?;
        if count > n {
            return Err(Status::INVALID_ARGUMENT);
        }
        Ok(count as usize)
    }

    fn presented(&mut self, rows: &mut [Sa2PresentedRow]) -> Result<usize, Status> {
        rows.iter_mut().for_each(|row| row.struct_size = size_of::<Sa2PresentedRow>() as u32);
        let n = capacity(rows.len());
        let mut count = 0u32;
        // SAFETY: `rows` holds `n` sized rows; the core writes at most `n`.
        Status(unsafe { (self.functions.presented)(self.context, rows.as_mut_ptr(), n, &mut count) }).check()?;
        if count > n {
            return Err(Status::INVALID_ARGUMENT);
        }
        Ok(count as usize)
    }

    fn pick_request(&mut self, frame_id: u64, x: u32, y: u32) -> Result<u64, Status> {
        let mut ticket = 0u64;
        // SAFETY: `ticket` is a live u64.
        Status(unsafe { (self.functions.pick_request)(self.context, frame_id, x, y, &mut ticket) }).check()?;
        Ok(ticket)
    }

    fn pick_poll(&mut self, ticket: u64) -> Result<Sa2PickRow, Status> {
        let mut row = Sa2PickRow::new();
        // SAFETY: `row` is a live, sized row.
        Status(unsafe { (self.functions.pick_poll)(self.context, ticket, &mut row) }).check()?;
        Ok(row)
    }

    fn recover(&mut self) -> (Status, String) {
        self.json(self.functions.recover)
    }

    fn counters(&self) -> Result<Sa2CounterValues, Status> {
        let mut row = Sa2CounterValues::new();
        // SAFETY: `row` is a live, sized row.
        Status(unsafe { (self.functions.counters)(self.context, &mut row) }).check()?;
        Ok(row)
    }

    fn device_status(&self) -> (Status, i32) {
        let mut reason = 0i32;
        // SAFETY: `reason` is a live i32.
        let status = Status(unsafe { (self.functions.device_status)(self.context, &mut reason) });
        (status, reason)
    }

    fn view_shape_set(&mut self, view: u64, shape: Option<&Sa2ViewShape>) -> Result<(), Status> {
        let row = shape.map(|s| Sa2ViewShape { struct_size: size_of::<Sa2ViewShape>() as u32, ..*s });
        let pointer = row.as_ref().map_or(std::ptr::null(), |r| r as *const Sa2ViewShape);
        // SAFETY: `pointer` is null or a live, sized row.
        Status(unsafe { (self.functions.view_shape_set)(self.context, view, pointer) }).check()
    }

    fn style_set(&mut self, revision: u64, piece_style: Option<&[u8]>, channel: u32) -> Result<(), Status> {
        if piece_style.is_some_and(|bytes| bytes.len() != PIECES) {
            return Err(Status::INVALID_ARGUMENT);
        }
        let pointer = piece_style.map_or(std::ptr::null(), <[u8]>::as_ptr);
        // SAFETY: `pointer` is null or PIECES readable bytes.
        Status(unsafe { (self.functions.style_set)(self.context, revision, pointer, channel) }).check()
    }

    fn view_style_set(&mut self, slot: u32, style: Option<&Sa2ViewStyle>) -> Result<(), Status> {
        let row = style.map(|s| Sa2ViewStyle { struct_size: size_of::<Sa2ViewStyle>() as u32, ..*s });
        let pointer = row.as_ref().map_or(std::ptr::null(), |r| r as *const Sa2ViewStyle);
        // SAFETY: `pointer` is null or a live, sized row.
        Status(unsafe { (self.functions.view_style_set)(self.context, slot, pointer) }).check()
    }

    fn last_error(&self) -> String {
        let mut buffer = vec![0u8; 1024];
        // SAFETY: the buffer holds 1024 bytes.
        unsafe { (self.functions.last_error)(self.context, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
        text(&buffer)
    }
}
