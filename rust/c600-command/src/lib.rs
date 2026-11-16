//! The command layer of LP3 (chapter 01 KN5; H-08; EN4's layer 3), headless.
//!
//! Every change to a session goes through [`CommandLayer::invoke`] and its gates (KN5.1). The layer reads H-08's rows
//! from the stored table ([`table`]), returns a receipt for every invocation ([`receipt`]), runs the word preview as a
//! job, commits a presented fresh preview through the ten commit steps of chapter 08 section 7, and undoes and redoes
//! through `session.undo` and `session.redo` with LI.9's action hint. The shell ([`shell`]) mints the user-action
//! tokens from a per-launch secret the runtime never receives (KN5.14, KN5.15) and resolves key bindings before the
//! invocation envelope (EV4.3), so a key bound to a kernel row reaches the engine with no interpreter step (ONE.48).
//! This crate depends on the kernel only: it has no path to the language runtime at all.
//!
//! Step 9's durable transaction sits behind [`store::Store`]. [`store::MemoryStore`] stands in until LP2's session store
//! (M6) is merged on main; receipts are durable only in that sense here.
//!
//! # Drafts
//!
//! The command table freezes at LP3 (PHS.53) and the IR later; these choices are drafts until then.
//!
//! - One session per layer, Jumble off, 600-cell-Full only. The jumbling engine joins when M5 is on main.
//! - `WordRef` is an inline literal recipe in 0.4's JSON form, `{"recipe": [...]}`. `ResolvedDraftRef` waits for the
//!   workspace store; a draft argument in any other form is refused with `KN5:input.argument_form`.
//! - `twist.apply`'s `rotation` is A6's exact request (KN3.7). Matching it to a cap's generator needs KN3's exact
//!   rotations, which the kernel does not build yet, so the row is refused with `KN5:input.row_not_built`, as is every
//!   other row this crate does not build yet. The loop uses the operation rows, which are kernel rows too.
//! - Codes chapter 13 does not register yet: `KN5:input.unknown_row`, `KN5:input.argument_form`,
//!   `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict` and `KN2:consistency.star_certificate`.
//! - The token travels beside the arguments in the envelope, not inside them, so it can bind the absolute arguments
//!   (LI.6). Its binding enters the request digest; its authenticator does not (LI.5).
//! - Jobs run when the host calls [`CommandLayer::run_jobs`]; `invoke` answers `running(job)` at once (KN5.2). The
//!   exclusive lane holds one job; a program lane's capacity is a host setting (LI.22), one by default. A view
//!   overlay's newer request supersedes a queued older one.
//! - A token is spent when it authenticates, so a refused commit consumes its press.
//! - The envelope carries its originating epoch (LI.5). A command ID the layer knows is answered by its outcome
//!   lookup, across a restart too; an unknown envelope from another epoch is refused with `KN2:stale.epoch_changed`,
//!   and its command ID stays used, so a new press takes a new ID.
//! - [`CommandLayer::present`] stands in for the adapter's RM.26 report of a frame the built-in chain drew (ONE.41).
//! - A program may replace the pending preview: its proposal runs inside a command the solver invoked (KN5.12 stays
//!   open; LI.38's working form).
//! - `operation.preview.cancel` changes no revision.
//! - Undo derives the inverse word through the kernel under the protection set in force (LI.26's normal permission
//!   path); no protection row is built, so no undo releases protection and none needs a confirmation yet. After an
//!   undo the revision's head is the journal's new head, the event below the undone one (KN5.21).
//! - The per-launch secret comes from the standard library's random hasher keys, the clock and the process ID, hashed
//!   with SHA-256. A product build needs an operating-system random source.

#![forbid(unsafe_code)]

pub mod layer;
pub mod receipt;
pub mod shell;
pub mod store;
pub mod table;
pub mod token;

pub use layer::{Caller, CommandLayer, Invocation, Preview, PreviewState, ProgramJob};
pub use receipt::{
    code, digest, ActionHint, CancelReason, CommitRecord, Completion, CompletionKind, Direction, Error, PreviewDeps,
    Receipt, ResultRef, Revision, Status,
};
pub use shell::{draft_args, word_draft, ArgSource, KeyBinding, Shell};
pub use store::{Fate, MemoryStore, Store, Transaction, TransactionKind};
pub use table::{Kind, Lane, Overlay, PreviewRule, Row, Table, UndoClass};
pub use token::{PreviewBinding, Secret, Token, TokenBinding};

/// The sources of the command layer and its loop runner, with the stored command table, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 10] = [
    ("c600-command/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-command/src/bin/c600-lp3-oracle.rs", include_bytes!("bin/c600-lp3-oracle.rs")),
    ("c600-command/src/layer.rs", include_bytes!("layer.rs")),
    ("c600-command/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-command/src/receipt.rs", include_bytes!("receipt.rs")),
    ("c600-command/src/shell.rs", include_bytes!("shell.rs")),
    ("c600-command/src/store.rs", include_bytes!("store.rs")),
    ("c600-command/src/table.rs", include_bytes!("table.rs")),
    ("c600-command/src/token.rs", include_bytes!("token.rs")),
    ("docs/progress/1.0/command-table.json", table::H08.as_bytes()),
];

/// The build string: the crate version, the SHA-256 of a `sha256sum`-style listing of [`SOURCES`], and the kernel's
/// build string.
pub fn build() -> String {
    let mut listing = String::new();
    for (name, bytes) in SOURCES {
        listing.push_str(&format!("{}  {name}\n", c600_digest::sha256_hex(bytes)));
    }
    let sources = c600_digest::sha256_hex(listing.as_bytes());
    format!("c600-command {} sources-sha256:{sources}; {}", env!("CARGO_PKG_VERSION"), c600_kernel::build())
}
