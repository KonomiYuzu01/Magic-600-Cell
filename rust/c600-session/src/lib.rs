//! The session store below the language (LP2, M6): one durable journal per session, which one process opens at a
//! time (KN4.1, the session lock in [`disk`]), revisions and their counters
//! (KN0.2), previews and their freshness (KN0.8, KN5.9 to KN5.11), commits with a user-action token (KN5.14 to
//! KN5.17), undo and redo with action hints (KN5.19, KN5.20, LI.9), receipts and outcome lookup (KN5.21 to KN5.23),
//! branches with one active line (ONE.75), read-only snapshots (ONE.76), workspace records with provenance blocks
//! (PS1.16) and program jobs on lanes under explicit budgets (KN7).
//!
//! The crate uses std and the platform layer `c600-platform`, which opens the session lock file (RPI.52). The memory
//! disk [`disk::MemDisk`] is for tests only, behind the crate feature `mem-disk`. Its row names, record shapes and
//! Rust shapes are M6 drafts (`rust/README.md`), except row names the command table holds since its freeze at LP3
//! (`workspace.set`, CM.17), and the session format is not the frozen migration format.
//!
//! [`JournalStore`] carries M6's durable semantics to c600-command's KN5 layer: it implements the layer's `Store` and
//! `Reopen` over the same journal ([`command_store`]). This crate depends on c600-command, never the reverse.

#![forbid(unsafe_code)]

pub mod command_store;
pub mod disk;
/// The host's document service: the live session as the first document and its history as a log (LP4-14 draft).
pub mod documents;
pub mod journal;
pub mod persist;
/// The command layer's recordings and playback states as workspace records of the session's journal (LP4-9 draft).
pub mod recorder;

mod jobs;
mod session;
mod store;
mod token;
mod types;
mod wire;

pub use command_store::JournalStore;
pub use session::{Options, Session, Shell};
pub use token::{hmac_sha256, Binding, PreviewBinding, Token};
pub use types::*;
pub use wire::request_digest;
