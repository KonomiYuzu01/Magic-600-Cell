//! The core's product calls as a safe Rust interface: one method per ABI 4 call, with slices where the C structs
//! carry pointers. [`crate::ffi::DllCore`] implements it over the C functions; [`crate::fake::FakeCore`] implements
//! it in Rust for headless tests.

use crate::abi::{Sa2CounterValues, Sa2FrameInput, Sa2FrameReport, Sa2PickRow, Sa2PollRow, Sa2PresentedRow, Status};

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

/// ABI 4's product calls on one attached context with a loaded scene. Every call comes from the context's thread.
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
}
