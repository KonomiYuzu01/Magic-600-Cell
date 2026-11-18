//! ABI 4's product API as Rust declarations: the constants and the `#[repr(C)]` rows of
//! `renderer/core/include/sa2_interop.h`, x64 layout. `tests/abi_layout.rs` checks every name, offset, size and
//! constant against the header.

use std::mem::size_of;

/// `SA2_ABI_VERSION`.
pub const ABI_VERSION: u32 = 4;
/// `SA2_SLOTS`: the labelled sticker slots of 600-cell-Full.
pub const SLOTS: usize = 259_800;
/// `SA2_PIECES`: the length of a pose index.
pub const PIECES: usize = 177_120;
/// `SA2_MAX_POSES`.
pub const MAX_POSES: usize = 177_120;
/// `SA2_POSE_GROUP_ORDER`: |K+|.
pub const POSE_GROUP_ORDER: i32 = 7200;

/// An `SA2_*` status code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Status(pub i32);

impl Status {
    pub const OK: Status = Status(0);
    pub const INVALID_ARGUMENT: Status = Status(1);
    pub const WRONG_STATE: Status = Status(2);
    pub const D3D12: Status = Status(3);
    pub const TIMEOUT: Status = Status(4);
    pub const DEVICE_REMOVED: Status = Status(5);
    pub const VERIFY: Status = Status(6);
    pub const UNSUPPORTED: Status = Status(7);
    pub const CHECK_FAILED: Status = Status(8);
    pub const IO: Status = Status(9);
    pub const UNAVAILABLE: Status = Status(10);

    /// The header's name without the `SA2_` prefix, or `unknown`.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "OK",
            1 => "E_INVALID_ARGUMENT",
            2 => "E_WRONG_STATE",
            3 => "E_D3D12",
            4 => "E_TIMEOUT",
            5 => "E_DEVICE_REMOVED",
            6 => "E_VERIFY",
            7 => "E_UNSUPPORTED",
            8 => "E_CHECK_FAILED",
            9 => "E_IO",
            10 => "E_UNAVAILABLE",
            _ => "unknown",
        }
    }

    /// `Ok(())` for `SA2_OK`, else the status.
    pub fn check(self) -> Result<(), Status> {
        if self == Status::OK {
            Ok(())
        } else {
            Err(self)
        }
    }
}

pub const TICKET_PENDING: i32 = 0;
pub const TICKET_READY: i32 = 1;
pub const TICKET_CANCELLED: i32 = 2;
pub const TICKET_FAILED: i32 = 3;

pub const CANCEL_BY_USER: i32 = 0;
pub const CANCEL_SUPERSEDED: i32 = 1;
pub const CANCEL_REPLACED_REVISION: i32 = 2;
pub const CANCEL_BUDGET: i32 = 3;

pub const PICK_PENDING: i32 = 0;
pub const PICK_READY: i32 = 1;
pub const PICK_STALE: i32 = 2;
pub const PICK_STALE_SURFACE: u32 = 1;
pub const PICK_STALE_VIEW: u32 = 2;
pub const PICK_STALE_TARGET: u32 = 3;
pub const PICK_STALE_ENGINE: u32 = 4;
pub const PICK_STALE_PUBLICATION: u32 = 5;

/// `SA2_FRAME_PREVIEW`.
pub const FRAME_PREVIEW: u32 = 1;

/// `sa2_engine_image`. Built only by [`crate::ffi`] from checked slices.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Sa2EngineImage {
    pub struct_size: u32,
    pub pose_count: u32,
    pub revision: u64,
    pub labels: *const u32,
    pub pose_index: *const i32,
    pub poses: *const f32,
    pub pose_lattice: *const i32,
    pub digest: [u8; 32],
}

/// `sa2_preparation`. Built only by [`crate::ffi`] from checked slices.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Sa2Preparation {
    pub struct_size: u32,
    pub reserved: u32,
    pub command: u64,
    pub view: u64,
    pub view_generation: u64,
    pub publication_generation: u64,
    pub device_epoch: u64,
    pub base_revision: u64,
    pub visibility: *const u8,
    pub interaction: *const u8,
    pub instances: *const u32,
    pub input_digest: [u8; 32],
}

/// The rows the core writes or reads by value: `new()` sets `struct_size` and zeroes the rest.
macro_rules! rows {
    ($($(#[$doc:meta])* $name:ident { $($field:ident: $ty:ty,)* })*) => {$(
        $(#[$doc])*
        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct $name {
            $(pub $field: $ty,)*
        }

        impl $name {
            pub fn new() -> $name {
                $name { struct_size: size_of::<$name>() as u32, ..Default::default() }
            }
        }
    )*};
}

rows! {
    /// `sa2_poll_row`.
    Sa2PollRow {
        struct_size: u32,
        state: i32,
        reason: i32,
        reserved: u32,
        resource_set: u64,
        superseded_by: u64,
    }

    /// `sa2_frame_input`.
    Sa2FrameInput {
        struct_size: u32,
        target: u32,
        flags: u32,
        reserved: u32,
        device_epoch: u64,
        surface: u64,
        surface_generation: u64,
        sequence: u64,
        engine_revision: u64,
        view: u64,
        view_generation: u64,
        publication_generation: u64,
        camera: u64,
        viewport_width: u32,
        viewport_height: u32,
        visibility: u64,
        interaction: u64,
        resource_set: u64,
        preview: u64,
        preview_base: u64,
        subjects_digest: [u8; 32],
        preview_literal_digest: [u8; 32],
        camera_matrix: [f32; 16],
    }

    /// `sa2_frame_report`.
    Sa2FrameReport {
        struct_size: u32,
        error: i32,
        fallback: u32,
        reserved: u32,
        frame_id: u64,
        device_epoch: u64,
        surface: u64,
        surface_generation: u64,
        sequence: u64,
        engine_revision: u64,
        view: u64,
        view_generation: u64,
        publication_generation: u64,
        gpu_time_ns: u64,
        view_vertex_evals: u64,
    }

    /// `sa2_presented_row`.
    Sa2PresentedRow {
        struct_size: u32,
        previews: u32,
        frame_id: u64,
        device_epoch: u64,
        surface: u64,
        sequence: u64,
        preview: u64,
    }

    /// `sa2_pick_row`.
    Sa2PickRow {
        struct_size: u32,
        state: i32,
        stale: u32,
        instance: u32,
        frame_id: u64,
        device_epoch: u64,
        surface: u64,
        surface_generation: u64,
        view: u64,
        view_generation: u64,
        publication_generation: u64,
        camera: u64,
        visibility: u64,
        interaction: u64,
        viewport_width: u32,
        viewport_height: u32,
        x: u32,
        y: u32,
    }

    /// `sa2_counter_values`.
    Sa2CounterValues {
        struct_size: u32,
        reserved: u32,
        view_vertex_evals_last: u64,
        view_vertex_evals_total: u64,
        bytes_uploaded: u64,
        frames_submitted: u64,
        frames_reported: u64,
        id_passes: u64,
        reports_dropped: u64,
    }
}
