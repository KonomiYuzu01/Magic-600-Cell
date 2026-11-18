//! C9's bridge from the display host to the S-A2 renderer core: [`RenderBackend`] implements c600-display's
//! [`c600_display::Backend`] over ABI 4 of `renderer/core/include/sa2_interop.h`.
//!
//! The bridge never loads the core. Whoever owns the process (the product shell) loads `sa2_interop.dll`, attaches a
//! context to its device and scene, and hands the bridge the context and a table of the core's functions
//! ([`ffi::CoreFunctions`]); Windows calls stay in `c600-platform` (CT-RPI-20). [`fake::FakeCore`] implements the
//! same calls in Rust, so the bridge runs headless.
//!
//! Layers: [`abi`] declares the header's constants and rows; [`api::CoreAbi`] is the safe call interface;
//! [`ffi::DllCore`] implements it over the C functions, and is the crate's only unsafe code; [`backend`] maps the
//! host's publications, frames and picks onto it.

#![deny(unsafe_code)]

pub mod abi;
pub mod api;
pub mod backend;
pub mod fake;
#[allow(unsafe_code)]
pub mod ffi;

pub use api::{CoreAbi, EngineImage, PoseArrays, PreparationInput};
pub use backend::{FrameReport, PickPoll, PickTicket, RenderBackend};
pub use fake::FakeCore;
pub use ffi::{CoreFunctions, DllCore};
