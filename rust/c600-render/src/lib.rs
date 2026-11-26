//! C9's bridge from the display host to the S-A2 renderer core: [`RenderBackend`] implements c600-display's
//! [`c600_display::Backend`] over ABI 5 of `renderer/core/include/sa2_interop.h`. ABI 5 adds route B (accepted for now
//! by the owner on 18 Nov 2026, pending three owner-machine checks): on a context the shell made with `sa2_create`, the
//! core presents into its own surface, confirms presentations and recovers its own device; the bridge needs no other
//! call for it, since the shell creates the context and attaches the surface.
//!
//! The bridge never loads the core and holds no foreign call. CT-RPI-01 keeps every member of the language workspace
//! free of unsafe code and foreign declarations, so the C function table and its implementation of [`api::CoreAbi`]
//! live outside the workspace, in `renderer/bridge` (`c600-render-ffi`), which the product shell links; the shell
//! loads `sa2_interop.dll` (CT-RPI-20 keeps Windows calls out of this workspace). [`fake::FakeCore`] implements the
//! same calls in Rust, so the bridge runs headless.
//!
//! Layers: [`abi`] declares the header's constants and rows; [`api::CoreAbi`] is the safe call interface; [`backend`]
//! maps the host's publications, frames and picks onto it.
//!
//! [`stage`] (C9 U5a) turns a user shader stage that LP2's stage checker admitted into HLSL under the interface
//! contract `m600-stage-hlsl/1`, and evaluates the same checked definition on the CPU in binary32 (RPI.74); the bin
//! `c600-stage-hlsl` writes the text and its record. Compiling the text is U5b's.

#![forbid(unsafe_code)]

pub mod abi;
pub mod api;
pub mod backend;
pub mod fake;
pub mod stage;

pub use api::{CoreAbi, EngineImage, PoseArrays, PreparationInput};
pub use backend::{DisplayTime, FrameReport, PaneSpec, PickPoll, PickTicket, RenderBackend};
pub use fake::FakeCore;
