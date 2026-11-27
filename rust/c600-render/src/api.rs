//! The core's product calls as a safe Rust interface: one method per ABI 9 product call, with slices where the C
//! structs carry pointers. `DllCore` in `renderer/bridge` (`c600-render-ffi`) implements it over the C functions;
//! [`crate::fake::FakeCore`] implements it in Rust for headless tests.

use crate::abi::{
    Sa2CounterValues, Sa2FrameInput, Sa2FrameReport, Sa2Pane, Sa2PaneRow, Sa2PickRow, Sa2PollRow, Sa2PresentedRow,
    Sa2SceneLook, Sa2ViewFocus, Sa2ViewLayout, Sa2ViewShape, Sa2ViewStyle, Status, StickerColours,
};

/// The pose path arrays of a posed engine state.
#[derive(Clone, Copy, Debug)]
pub struct PoseArrays<'a> {
    /// One pose index per piece (`PIECES` entries).
    pub pose_index: &'a [i32],
    /// `pose_lattice.len()` row-major 4x4 matrices.
    pub poses: &'a [f32],
    pub pose_lattice: &'a [i32],
}

/// `sa2_engine_image`'s content.
#[derive(Clone, Copy, Debug)]
pub struct EngineImage<'a> {
    pub revision: u64,
    /// `SLOTS` labels by slot.
    pub labels: &'a [u32],
    pub pose: Option<PoseArrays<'a>>,
    pub digest: [u8; 32],
}

/// `sa2_preparation`'s content.
#[derive(Clone, Copy, Debug)]
pub struct PreparationInput<'a> {
    pub command: u64,
    pub view: u64,
    pub view_generation: u64,
    pub publication_generation: u64,
    pub device_epoch: u64,
    pub base_revision: u64,
    /// `SLOTS` bytes, 0 or 1.
    pub visibility: &'a [u8],
    /// `SLOTS` bytes, 0 or 1.
    pub interaction: &'a [u8],
    /// `SLOTS` instance IDs by slot; 0 = none.
    pub instances: &'a [u32],
    pub input_digest: [u8; 32],
}

/// ABI 5's product calls on one context with a loaded scene, one call at a time: on a `sa2_attach` context from
/// Godot's render thread, on a `sa2_create` context (route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks) from any thread.
pub trait CoreAbi {
    /// `sa2_describe`: the support report's JSON.
    fn describe(&self) -> Result<String, Status>;
    /// `sa2_engine_adopt`.
    fn engine_adopt(&mut self, image: &EngineImage<'_>) -> Result<u64, Status>;
    /// `sa2_prepare`: the ticket.
    fn prepare(&mut self, input: &PreparationInput<'_>) -> Result<u64, Status>;
    /// `sa2_poll`.
    fn poll(&mut self, ticket: u64) -> Result<Sa2PollRow, Status>;
    /// `sa2_activate`.
    fn activate(&mut self, resource_set: u64, stamp_digest: &[u8; 32]) -> Result<(), Status>;
    /// `sa2_cancel`.
    fn cancel(&mut self, ticket: u64, reason: i32, superseded_by: u64) -> Result<(), Status>;
    /// `sa2_retire`.
    fn retire(&mut self, resource_set: u64) -> Result<(), Status>;
    /// `sa2_frame_submit`: the frame ID.
    fn frame_submit(&mut self, input: &Sa2FrameInput) -> Result<u64, Status>;
    /// `sa2_frame_reports`: the number of rows written.
    fn frame_reports(&mut self, rows: &mut [Sa2FrameReport]) -> Result<usize, Status>;
    /// `sa2_presented`: the number of rows written.
    fn presented(&mut self, rows: &mut [Sa2PresentedRow]) -> Result<usize, Status>;
    /// `sa2_pick_request`: the pick ticket.
    fn pick_request(&mut self, frame_id: u64, x: u32, y: u32) -> Result<u64, Status>;
    /// `sa2_pick_poll`.
    fn pick_poll(&mut self, ticket: u64) -> Result<Sa2PickRow, Status>;
    /// `sa2_recover`: the status and the JSON it wrote, which it writes with `SA2_E_UNAVAILABLE` too.
    fn recover(&mut self) -> (Status, String);
    /// `sa2_counters`.
    fn counters(&self) -> Result<Sa2CounterValues, Status>;
    /// `sa2_device_status`: the status and `GetDeviceRemovedReason`.
    fn device_status(&self) -> (Status, i32);
    /// `sa2_last_error` of the context.
    fn last_error(&self) -> String;
    /// ABI 6's `sa2_view_shape_set`: the shape of `view`'s frames, or `None` for the defaults (ABI 5's constants).
    fn view_shape_set(&mut self, view: u64, shape: Option<&Sa2ViewShape>) -> Result<(), Status>;
    /// ABI 9's `sa2_view_focus_set`: the focus of `view`'s frames, or `None` for the defaults. A core without it
    /// answers `UNSUPPORTED`.
    fn view_focus_set(&mut self, view: u64, focus: Option<&Sa2ViewFocus>) -> Result<(), Status> {
        let _ = (view, focus);
        Err(Status::UNSUPPORTED)
    }
    /// ABI 10's `sa2_view_layout_set`: the layout of `view`'s frames, or `None` for the chain. A core without it
    /// answers `UNSUPPORTED`.
    fn view_layout_set(&mut self, view: u64, layout: Option<&Sa2ViewLayout>) -> Result<(), Status> {
        let _ = (view, layout);
        Err(Status::UNSUPPORTED)
    }
    /// ADR-D02 B18's `sa2_transform_set` (added to ABI 10 before any release): the adopted revision's transform, the
    /// plane's rows u and v, the angle, the moving set (`PIECES` bytes of 0 or 1) and the phase, or `None` to clear
    /// it. A core without it answers `UNSUPPORTED`.
    fn transform_set(
        &mut self,
        revision: u64,
        plane: &[[f32; 4]; 2],
        angle: f32,
        moving: Option<&[u8]>,
        phase: f32,
    ) -> Result<(), Status> {
        let _ = (revision, plane, angle, moving, phase);
        Err(Status::UNSUPPORTED)
    }
    /// ABI 7's `sa2_style_set`: one channel of the adopted revision's piece style (`PIECES` bytes), or `None` to clear
    /// it.
    fn style_set(&mut self, revision: u64, piece_style: Option<&[u8]>, channel: u32) -> Result<(), Status>;
    /// ABI 7's `sa2_view_style_set`: style slot 1 to 4, or `None` to clear it.
    fn view_style_set(&mut self, slot: u32, style: Option<&Sa2ViewStyle>) -> Result<(), Status>;
    /// ABI 8's `sa2_frame_submit_panes`: a composite frame of 1 to `PANES_MAX` panes; the frame ID.
    fn frame_submit_panes(&mut self, input: &Sa2FrameInput, panes: &[Sa2Pane]) -> Result<u64, Status>;
    /// ABI 8's `sa2_frame_panes`: a produced composite frame's panes, the first `rows.len()` of them written; the
    /// frame's pane count.
    fn frame_panes(&mut self, frame_id: u64, rows: &mut [Sa2PaneRow]) -> Result<usize, Status>;
    /// ABI 10's `sa2_sticker_colours_set`: home-cell RGB triples, or `None` for the core's hues.
    fn sticker_colours_set(&mut self, rgb: Option<&StickerColours>) -> Result<(), Status> {
        let _ = rgb;
        Err(Status::UNSUPPORTED)
    }
    /// ABI 10's `sa2_scene_look_set`: the scene's look, or `None` for the core's defaults.
    fn scene_look_set(&mut self, look: Option<&Sa2SceneLook>) -> Result<(), Status> {
        let _ = look;
        Err(Status::UNSUPPORTED)
    }
}
