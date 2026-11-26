//! ABI 9's product API as Rust declarations: the constants and the `#[repr(C)]` rows of
//! `renderer/core/include/sa2_interop.h`, x64 layout. `tests/abi_layout.rs` checks every name, offset, size and
//! constant against the header.

use std::mem::size_of;

/// `SA2_ABI_VERSION`. ABI 6 adds a view's shape ([`Sa2ViewShape`]); ABI 7 the piece style and the style slots
/// ([`Sa2ViewStyle`]); ABI 8 the composite frame ([`Sa2Pane`], [`Sa2PaneRow`]); ABI 9 a view's focus
/// ([`Sa2ViewFocus`]).
pub const ABI_VERSION: u32 = 9;
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

// ABI 5, route B (accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks).
/// `SA2_TARGET_SURFACE`: the frame target that names the attached surface of a `sa2_create` context.
pub const TARGET_SURFACE: u32 = 0xFFFF_FFFF;
/// `SA2_SURFACE_SYNC_INTERVAL`.
pub const SURFACE_SYNC_INTERVAL: u32 = 1;
/// `SA2_SURFACE_BUFFERS`.
pub const SURFACE_BUFFERS: u32 = 3;
/// `SA2_SURFACE_MAX_PX`.
pub const SURFACE_MAX_PX: u32 = 16384;
/// `SA2_RING_SLOTS`: the ring-slot frame targets of the Godot-texture route are 0 to 2.
pub const RING_SLOTS: u32 = 3;

// ABI 6: a view's shape (`sa2_view_shape`); the defaults are ABI 5's constants.
/// `SA2_SHAPE_DEFAULT_CELL_SHRINK`.
pub const SHAPE_DEFAULT_CELL_SHRINK: f32 = 0.76;
/// `SA2_SHAPE_DEFAULT_STICKER_SHRINK`.
pub const SHAPE_DEFAULT_STICKER_SHRINK: f32 = 0.82;
/// `SA2_SHAPE_DEFAULT_D4`.
pub const SHAPE_DEFAULT_D4: f32 = 1.18;
/// `SA2_SHAPE_DEFAULT_ZOOM`.
pub const SHAPE_DEFAULT_ZOOM: f32 = 1.15;
/// `SA2_SHAPE_D4_MAX`.
pub const SHAPE_D4_MAX: f32 = 64.0;
/// `SA2_SHAPE_ZOOM_MAX`.
pub const SHAPE_ZOOM_MAX: f32 = 64.0;

// ABI 9: a view's focus (`sa2_view_focus`).
/// `SA2_PROJECTION_PERSPECTIVE`.
pub const PROJECTION_PERSPECTIVE: u32 = 0;
/// `SA2_PROJECTION_ORTHOGRAPHIC`.
pub const PROJECTION_ORTHOGRAPHIC: u32 = 1;
/// `SA2_FOCUS_NONE`.
pub const FOCUS_NONE: u32 = 0xFFFF_FFFF;

// ABI 7: the piece style (`sa2_style_set`) and the style slots (`sa2_view_style`).
/// `SA2_STYLE_CHANNEL_FILL`.
pub const STYLE_CHANNEL_FILL: u32 = 0;
/// `SA2_STYLE_CHANNEL_OUTLINE`.
pub const STYLE_CHANNEL_OUTLINE: u32 = 1;
/// `SA2_STYLE_INDEX`: a style byte's palette entry.
pub const STYLE_INDEX: u8 = 0x0F;
/// `SA2_STYLE_DIM`: a fill byte's dim-set bit.
pub const STYLE_DIM: u8 = 0x80;
/// `SA2_STYLE_SLOTS`: style slots 1 to 4.
pub const STYLE_SLOTS: u32 = 4;
/// `SA2_STYLE_PALETTE`.
pub const STYLE_PALETTE: u32 = 16;
/// `SA2_STYLE_TINT_FILL`.
pub const STYLE_TINT_FILL: u32 = 1;
/// `SA2_STYLE_TINT_OUTLINE`.
pub const STYLE_TINT_OUTLINE: u32 = 2;
/// `SA2_STYLE_DIM_NONE`.
pub const STYLE_DIM_NONE: u32 = 0;
/// `SA2_STYLE_DIM_MARKED`.
pub const STYLE_DIM_MARKED: u32 = 1;
/// `SA2_STYLE_DIM_UNMARKED`.
pub const STYLE_DIM_UNMARKED: u32 = 2;
/// `SA2_STYLE_OUTLINE_MAX`.
pub const STYLE_OUTLINE_MAX: f32 = 0.5;
/// `SA2_STYLE_MATRIX_MAX`.
pub const STYLE_MATRIX_MAX: f32 = 8.0;

// ABI 8: the composite frame (`sa2_frame_submit_panes`).
/// `SA2_PANES_MAX`: the panes of one composite frame.
pub const PANES_MAX: u32 = 4;

/// `sa2_engine_image`. Built only by `c600-render-ffi` (`renderer/bridge`) from checked slices.
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

/// `sa2_preparation`. Built only by `c600-render-ffi` (`renderer/bridge`) from checked slices.
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
        display_qpc: u64,
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

    /// `sa2_pane_row` (ABI 8): one pane of a produced composite frame.
    Sa2PaneRow {
        struct_size: u32,
        pane: u32,
        view: u64,
        view_generation: u64,
        vertex_evals: u64,
    }

    /// `sa2_view_shape` (ABI 6).
    Sa2ViewShape {
        struct_size: u32,
        cell_shrink: f32,
        sticker_shrink: f32,
        d4: f32,
        zoom: f32,
        reserved: u32,
    }

    /// `sa2_view_focus` (ABI 9).
    Sa2ViewFocus {
        struct_size: u32,
        projection: u32,
        focus_cell: u32,
        focus_cos: f32,
        spread: f32,
        reserved: u32,
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

/// `sa2_pane` (ABI 8): one pane of a composite frame, its rectangle in target pixels. It has no `struct_size`, so it
/// is not in `rows!`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sa2Pane {
    pub view: u64,
    pub view_generation: u64,
    /// 0: every slot drawn, nothing pickable.
    pub resource_set: u64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Column-major, as [`Sa2FrameInput::camera_matrix`].
    pub camera_matrix: [f32; 16],
    /// 0: no style; else a style slot 1 to [`STYLE_SLOTS`].
    pub style: u32,
    /// Must be 0.
    pub flags: u32,
}

impl Sa2Pane {
    /// Whether `self` and `other` share a pixel.
    pub fn overlaps(&self, other: &Sa2Pane) -> bool {
        let (ax, ay, bx, by) = (i64::from(self.x), i64::from(self.y), i64::from(other.x), i64::from(other.y));
        ax < bx + i64::from(other.width)
            && bx < ax + i64::from(self.width)
            && ay < by + i64::from(other.height)
            && by < ay + i64::from(self.height)
    }

    /// Whether the rectangle is not empty and lies inside a target of `size`.
    pub fn inside(&self, size: (u32, u32)) -> bool {
        self.width >= 1
            && self.height >= 1
            && self.x >= 0
            && self.y >= 0
            && i64::from(self.x) + i64::from(self.width) <= i64::from(size.0)
            && i64::from(self.y) + i64::from(self.height) <= i64::from(size.1)
    }
}

/// `sa2_view_style` (ABI 7): a style slot. Not in `rows!`, whose `Default` derive stops at 32-element arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sa2ViewStyle {
    pub struct_size: u32,
    pub tint_mode: u32,
    pub dim_mode: u32,
    pub dim_amount: f32,
    pub outline_width: f32,
    pub palette: [f32; 64],
    pub matrix: [f32; 9],
    pub reserved: u32,
}

impl Default for Sa2ViewStyle {
    fn default() -> Sa2ViewStyle {
        Sa2ViewStyle::new()
    }
}

impl Sa2ViewStyle {
    /// `struct_size` set, no tint, no dim, an empty palette and the identity matrix: a slot that changes nothing.
    pub fn new() -> Sa2ViewStyle {
        Sa2ViewStyle {
            struct_size: size_of::<Sa2ViewStyle>() as u32,
            tint_mode: 0,
            dim_mode: STYLE_DIM_NONE,
            dim_amount: 0.0,
            outline_width: 0.0,
            palette: [0.0; 64],
            matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            reserved: 0,
        }
    }

    /// The header's checks: known mode bits, every value finite and inside its range.
    pub fn in_range(&self) -> bool {
        let within = |v: f32, low: f32, high: f32| v.is_finite() && v >= low && v <= high;
        self.tint_mode & !(STYLE_TINT_FILL | STYLE_TINT_OUTLINE) == 0
            && self.dim_mode <= STYLE_DIM_UNMARKED
            && within(self.dim_amount, 0.0, 1.0)
            && within(self.outline_width, 0.0, STYLE_OUTLINE_MAX)
            && self.palette.iter().all(|&v| within(v, 0.0, 1.0))
            && self.matrix.iter().all(|&v| within(v, -STYLE_MATRIX_MAX, STYLE_MATRIX_MAX))
    }
}

/// Whether every byte of a piece style channel uses only that channel's bits (ABI 7).
pub fn piece_style_valid(bytes: &[u8], channel: u32) -> bool {
    let allowed = match channel {
        STYLE_CHANNEL_FILL => STYLE_INDEX | STYLE_DIM,
        STYLE_CHANNEL_OUTLINE => STYLE_INDEX,
        _ => return false,
    };
    bytes.len() == PIECES && bytes.iter().all(|&b| b & !allowed == 0)
}

impl Sa2ViewShape {
    /// The shape ABI 5 draws every view with, as `sa2_view_shape` takes it.
    pub fn defaults() -> Sa2ViewShape {
        Sa2ViewShape {
            cell_shrink: SHAPE_DEFAULT_CELL_SHRINK,
            sticker_shrink: SHAPE_DEFAULT_STICKER_SHRINK,
            d4: SHAPE_DEFAULT_D4,
            zoom: SHAPE_DEFAULT_ZOOM,
            ..Sa2ViewShape::new()
        }
    }

    /// The header's ranges: each value finite, cell and sticker shrink in (0, 1], d4 in (1, `SHAPE_D4_MAX`] and zoom
    /// in (0, `SHAPE_ZOOM_MAX`].
    pub fn in_range(&self) -> bool {
        let within = |v: f32, low: f32, high: f32| v.is_finite() && v > low && v <= high;
        within(self.cell_shrink, 0.0, 1.0)
            && within(self.sticker_shrink, 0.0, 1.0)
            && within(self.d4, 1.0, SHAPE_D4_MAX)
            && within(self.zoom, 0.0, SHAPE_ZOOM_MAX)
    }
}

impl Sa2ViewFocus {
    /// The focus every view draws with until one is set: perspective, no pole, spread 1.
    pub fn defaults() -> Sa2ViewFocus {
        Sa2ViewFocus {
            projection: PROJECTION_PERSPECTIVE,
            focus_cell: FOCUS_NONE,
            focus_cos: -1.0,
            spread: 1.0,
            ..Sa2ViewFocus::new()
        }
    }

    /// The header's ranges: a known projection, a cell below 600 or `FOCUS_NONE`, focus_cos in [-1, 1] and spread in
    /// [1, 3], each finite.
    pub fn in_range(&self) -> bool {
        (self.projection == PROJECTION_PERSPECTIVE || self.projection == PROJECTION_ORTHOGRAPHIC)
            && (self.focus_cell == FOCUS_NONE || self.focus_cell < 600)
            && self.focus_cos.is_finite()
            && (-1.0..=1.0).contains(&self.focus_cos)
            && self.spread.is_finite()
            && (1.0..=3.0).contains(&self.spread)
    }
}
