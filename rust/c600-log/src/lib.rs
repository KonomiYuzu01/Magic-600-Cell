//! The 1.0 log file of 600 Cell Script: its format, its compiler (LP4-12) and its interchange with 0.4's proof logs
//! and MPUltimate logs (LP4-13). See `rust/README.md` for status and what is not built.
//!
//! - [`Log`] is what a log holds (ADR-D01.80 as revision 6's R-3 amends it): the model identity, the start state, the
//!   move and layout histories with abandoned branches and partial undos, the state at save with its full-state hash,
//!   the view, layout and workspace, the open documents with their saved digests and unsaved contents, the embedded
//!   macros, Library entries and scripts with their author classes, and the preferences, recorded and never applied.
//! - [`write`] and [`read`] are its record: FP.11 with format `c600-log-draft`, version 1, null provenance, in FP.5's
//!   `c600-json-1` or FP.7's stored gzip, sealed by a content digest. The extension is [`EXTENSION`] (`.c600log`).
//! - [`compile`] checks a log's bytes (ADR-D01.81, revision 6's R-2): format, `model_match`, every move with its legal
//!   witness, the final full-state hash, the embedded macros, and the embedded scripts through the editor services.
//!   It never changes live state and never runs a script; a log whose first four gates did not pass binds no state.
//! - [`Recorder`] writes a log action by action with the kernel computing each state.
//! - [`c600`] and [`mpult`] check, import and export 0.4's `C600-STUDIO-SESSION-v1` and MPUltimate v1 logs of
//!   600-cell-Full with 0.4's limits.

#![forbid(unsafe_code)]

pub mod c600;
mod compile;
mod log;
pub mod mpult;
mod problem;
mod record;
pub mod wire;

pub use compile::{compile, Compiled, GateResult, ScriptCheck};
pub use log::{
    identity_value, read, record, write, Author, Commit, Cursor, CursorAction, DocumentIdentity, Extension,
    Interchange, LayoutAction, LayoutEntry, LayoutStep, LibraryEntry, Log, Macro, MoveAction, MoveEntry, MpultOrigin,
    OpenDocument, Origin, PartialUndo, Saved, Script, Start, WorkspaceStep, EXTENSION, FORMAT, VERSION,
};
pub use problem::{code, Gate, Problem};
pub use record::{recipe_value, Recorder};
