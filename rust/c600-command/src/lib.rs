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
//! Step 9's durable transaction sits behind [`store::Store`], and [`CommandLayer::open`] replays a store's history
//! through the kernel ([`store::Reopen`]) to restore the revision, the protected set, the undo and redo history and the
//! durable receipts. LP2's session store implements both over KN4's journal (`c600_session::JournalStore`, which
//! depends on this crate and never the reverse); [`store::MemoryStore`] keeps the same contract in memory for the
//! tests and the oracle loop.
//!
//! # Owner decisions
//!
//! The owner decided these readings of the frozen text under PHS.52 on 16 November 2026
//! (`docs/wiki/decisions/owner-decisions-2026-11-16-phs52.md`). The frozen text itself changes with the amendment LP3
//! makes at its command-table freeze.
//!
//! - Ruling 1: a press whose preview ended is refused `KN5:stale.no_fresh_preview` when the slot is empty and
//!   `KN5:stale.preview_replaced` when it holds another preview; a token for a preview never presented is
//!   `no_fresh_preview`. LI.33's vector, which answers `preview_replaced` after the pressed preview was committed, is
//!   amended to follow this ruling.
//! - Ruling 4 (KN5.21, KN5.23): the retry lookup keeps its frozen place before authentication (chapter 08 section 7),
//!   and the host stamps the caller from the channel the envelope came on. The envelope ([`Invocation`]) names no
//!   caller; [`CommandLayer::invoke`] takes the channel's caller beside it. Command IDs are scoped per caller (the
//!   shell, the runtime; no host caller is built). A record is keyed by the scope and the ID, and the request digest
//!   covers the stamped caller (LI.5's amendment says so). A refusal of an envelope that does not authenticate (a bad
//!   authenticator, a missing token, or a caller that may not use the row or hold a token) is retained for the epoch
//!   under the envelope's full digest, authenticator included, which is its receipt's `request`; it never attaches to
//!   the command ID. Every other answer attaches to the ID. An envelope whose request digest equals a recorded one is
//!   answered as its retry whatever its authenticator, and commits nothing.
//! - EN4's layer interfaces (LI) are frozen with the architecture (owner, 15 November 2026), so a change to LI follows
//!   PHS.52. Where LI is silent, the layer's choices below are drafts.
//!
//! # Drafts
//!
//! The command table freezes at LP3 (PHS.53) and the IR later; these choices are drafts until then.
//!
//! - One session per layer, Jumble off, 600-cell-Full only. The jumbling engine joins when M5 is on main.
//! - `WordRef` is an inline literal recipe in 0.4's JSON form, `{"recipe": [...]}`, and `ResolvedDraftRef` is
//!   `{"record": name}`, a workspace record that holds a word's letters (M6's form); a draft argument in any other form
//!   is refused with `KN5:input.argument_form`. A record draft's digest is M6's `{"letters", "source", "reads"}`, with
//!   the record's content digest and version in `reads` (KN0.8).
//! - The workspace store ([`workspace`]) holds M6's records: `workspace.set` ([`RECORD_ROW`]) takes `record`, `payload`
//!   and `provenance`, the provenance block's canonical JSON as text. H-08 has no such row yet, so [`Table::with_drafts`]
//!   adds it, and the layer refuses it to the runtime with `LG3:capability.not_granted`, as M6 does. The layer cannot
//!   read a provenance block: the host's [`Provenance`] reader checks it at the write (PS1.16) and admits a record's
//!   word at `operation.preview` in this session (KN5.26), and without a reader both are refused.
//! - `twist.apply`'s `rotation` is A6's exact request (KN3.7). Matching it to a cap's generator needs KN3's exact
//!   rotations, which the kernel does not build yet, so the row is refused with `KN5:input.row_not_built`, as is every
//!   other row this crate does not build yet. The loop uses the operation rows, which are kernel rows too.
//! - Codes chapter 13 does not register yet: `KN5:input.unknown_row`, `KN5:input.argument_form`,
//!   `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict`, `KN4:input.session_exists` and
//!   `KN2:consistency.star_certificate`.
//! - The token travels beside the arguments in the envelope, not inside them, so it can bind the absolute arguments
//!   (LI.6). Its binding enters the request digest; its authenticator does not (LI.5).
//! - Jobs run when the host calls [`CommandLayer::run_jobs`]; `invoke` answers `running(job)` at once (KN5.2). The
//!   exclusive lane holds one job; a program lane's capacity is a host setting (LI.22), one by default. A view
//!   overlay's newer request supersedes a queued older one.
//! - A token is spent when it authenticates, so a refused commit consumes its press.
//! - The envelope carries its originating epoch (LI.5). A command ID the layer knows is answered by its outcome
//!   lookup, across a restart too; an unknown envelope from another epoch is refused with `KN2:stale.epoch_changed`,
//!   and when it authenticates its command ID stays used, so a new press takes a new ID.
//! - [`CommandLayer::present`] reads the adapter's RM.26 reports ([`frame`]) in the order of their stamps, by device
//!   epoch, then sequence. A report that is not newer than the newest is refused and changes nothing. The pending
//!   preview is presented while the newest frame shows it through the built-in chain, and a frame that shows no
//!   preview, another preview or a user stage's drawing (RPI.76) ends the presentation. A word commit's token binds the
//!   stamp of the frame that presented its preview, and the commit is refused with `KN5:stale.no_fresh_preview` unless
//!   that frame belongs to the preview's current presentation: on one device epoch, surface and surface generation,
//!   from the first frame that showed it to the newest. A restart forgets the frame reports. [`HeadlessFrames`] stamps
//!   the frames of a headless adapter.
//! - A program may replace the pending preview: its proposal runs inside a command the solver invoked (KN5.12 stays
//!   open; LI.38's working form).
//! - `operation.preview.cancel` changes no revision.
//! - Undo and redo walk M6's move tree through the original permission path (LI.26): a move's inverse is derived
//!   through the kernel under the protected set in force, from a head whose hash matches its node; the undo of a
//!   protection change restores the set before it and needs `confirm` when it releases an orbit. After an undo the
//!   revision's head is the node the undo returns to, not the undo's own event (KN5.21).
//! - Draft ruling 2: an undo or redo checks its hint before its bound revisions, so a press raced by any durable action
//!   is `KN5:stale.undo_superseded`.
//! - Draft ruling 3: no receipt and no lookup answers `accepted`. A record is written once the gates answered, and a
//!   refusal never overwrites a stored record.
//! - Draft ruling 5: a token whose authenticator fails is `KN5:auth.token_invalid`; one that verifies but names another
//!   session or epoch is `KN5:stale.token_mismatch`.
//! - Draft ruling 7: `session.checkpoint.save` follows `command-table.json`, a commit row that takes a token; it is not
//!   built yet. M6's `session.checkpoint` takes no token.
//! - A commit token binds the SHA-256 of the kernel's identity block ([`identity_json`]) and the preview's
//!   dependencies (KN0.8); an undo or redo token binds the state and protection revisions and the hint (LI.6).
//! - `confirm` is an optional boolean argument of a row with the `confirm` gate or an action guard.
//! - A new session starts in epoch 1. Every open and every restart draws a fresh random epoch of at least 2 that the
//!   session never had, also under an injected secret (L10). The shell's command IDs are `prefix/n` in epoch 1 and
//!   `prefix/epoch/n` after it, so a new shell never meets an earlier epoch's IDs; two shells with one prefix in one
//!   epoch would collide. Preview and job IDs restart after a reopen; the epoch in every binding keeps them apart.
//! - [`CommandLayer::outcome`] is the shell's lookup and answers `None` for an ID the shell never used in this layer's
//!   life; [`CommandLayer::kernel_outcome`] is `kernel.outcome` for any caller.
//! - `protection.orbit.set` is a workspace row with no token; the runtime may not invoke it.
//! - Strict protection in the command layer (KN2.4, KN2.7, LB2.3). `protection.policy.set` ([`POLICY_ROW`]) takes
//!   `policy`, `net` or `strict`, as H-08 has it; the session starts Net ([`CommandLayer::policy`]). Under Strict a
//!   preview is built with the kernel's `derive_strict`, step 8 judges the preview's recipe again with
//!   `strict_protection`, and an undo or redo derives its path with `derive_strict`, so every executed literal prefix
//!   is judged; a Strict preview's summary adds `mode` and `first_refused_prefix`, and a Net preview's summary is
//!   unchanged. The row is a workspace row with no token and, as H-08 gives it no gate, no confirmation in either
//!   direction; the runtime may not invoke it. A policy change advances the workspace and protection revisions, so a
//!   pending preview built under the old policy is stale (H-08's note), and it is undoable as a workspace inverse. The
//!   journal names it `policy`, and it and its undo or redo carry the policy after the action
//!   (`Transaction::policy`).
//! - An action that does not move the puzzle (a protection or policy change, a record write, the undo or redo of any
//!   of them, a workspace completion) keeps the state revision, the preview slot and the jobs (KN5.18).
//! - A record write answers `completed(workspace)`, and its undo and redo answer `committed` with equal puzzle hashes,
//!   advancing the workspace revision and the record's version (KN5.20). An undo of a record's first write leaves the
//!   record with no value. The journal keeps the record's value after each of these actions (`Transaction::record`).
//! - A workspace overlay's completion is one durable action with its receipt (KN5.23) but is not undoable: the layer
//!   holds no inverse of a program's write. A rolled-back transaction's `failed` receipt is transient.
//! - A built-in tool's entry may serve a stored row ([`CommandLayer::serve_row`]; LP3's C6, its first common need).
//!   - An invocation of a served row admits a program job for the entry, as for an overlay row.
//!   - The row keeps H-08's kind, lane, undo class, gates and argument types. The runtime may invoke it, as it may an
//!     overlay row.
//!   - A row on the `none` lane has no occupancy limit (KN7.2), so a full program lane does not hold it back, and
//!     nothing in the layer bounds how many of its jobs wait. A commit still ends its job, as it ends an overlay
//!     row's (KN5.18), although KN7.2 gives the `none` lane no background work to cancel. Whether such a row should
//!     count against the program lane, and whether a workspace entry that reads no snapshot should keep its job
//!     across a commit as file jobs do, is LP3's call.
//!   - H-08 marks the representations tool's rows `job: false`. A served row still answers `running` first, since
//!     its entry runs in the runtime's process (LQ9), and completes when the host reports the entry's result.
//!   - The layer checks the arguments' forms, as below.
//!   - The layer does not read a row's `contexts`, for a served row as for any other: the shell's key route resolves
//!     a binding in the focused context.
//!   - The layer serves only a view, workspace, query or file row on the `none`, `view` or `program` lane, with no
//!     gate, preview rule, action guard, token or undo entry. It can check such a row's admission and completion
//!     without the entry.
//!   - Any other row is refused with `KN5:input.overlay_row_kind`. A row the layer builds itself, or one already
//!     served, is refused with `KN5:input.overlay_row_conflict`.
//!   - A served workspace row completes like a workspace overlay, as above.
//!   - Serving is not journaled. After a reopen the host serves its rows again, as it registers its overlays again.
//! - Argument forms (LP3's C6, its third common need).
//!   - An argument of an optional type, one ending in `?` (H-08), may be left out of any row. Every other argument
//!     must be given, and no other name may be. None of the layer's own rows takes an optional argument. The names
//!     are checked before the row's arm, so a row the layer does not build is refused with `KN5:input.row_not_built`
//!     once its names fit, with an optional argument left out or given.
//!   - For a row it admits as a program job, a served row or an overlay row, the layer checks the forms of the types
//!     it knows before it admits the job. `Nat` is an integer of at least 0. `Text` is a string. A choice of words,
//!     such as `body|complete`, is one of its words. `CellId` is an integer from 0 to 599, as the kernel's model
//!     numbers the cells, and `CellIdPair` is a list of two. `RevisionRef` is KN0.2's nine-key object
//!     ([`Revision::to_json`]): its text fields are text and its counters naturals, in any mode. An optional type has
//!     its type's form when given.
//!   - A malformed argument is refused with `KN5:input.argument_form`. After every form check, a well-formed
//!     `RevisionRef` that is not the layer's current revision in every field is refused with
//!     `KN5:stale.revision_changed` (EV4.14), so a malformed request is never answered as stale. Both refusals come
//!     before the lane's capacity is checked and take no job.
//!   - The forms of the other types are not written down yet, and they pass unchecked: among them `FilterRef`,
//!     `FilterPreviewRef` and `CanonicalObjectRefs`, which the representations tool's rows take, and a union of types
//!     such as `ResolvedDraftRef|WordRef`. The layer's own rows check their arguments where they are built.
//! - Job output (LP3's C6, its second common need). The host reports a program job's output as the runtime's sinks
//!   gave it ([`CommandLayer::finish_program_job_with`], [`JobOutput`]): the value the entry returned, the views it
//!   published (`publish_view`) and the workspace records it wrote (`write_workspace`), each record with its
//!   provenance block. [`CommandLayer::finish_program_job`] reports a value alone, as before.
//!   - The completion's result reference carries [`JobOutput::digest`]: the value's own digest when the job published
//!     no view and wrote no record, so a value alone keeps its digest, and otherwise the digest of the records, the
//!     value and the views together. The layer keeps no view or value; the host keeps the output.
//!   - Only a workspace row's job writes records. A record from any other row's job fails the job with
//!     `LG3:capability.not_granted`. Views pass from every row.
//!   - The host's provenance reader checks every record's block, as for `workspace.set` (PS1.16). A block it refuses
//!     fails the job with the reader's error, and without a reader the job fails with `KN5:input.row_not_built`.
//!   - A job whose output fails a check writes nothing.
//!   - A workspace row's records are written in its completion's one durable action, in the order of their names:
//!     each record's version advances, and the workspace revision advances once. The journal keeps them
//!     (`Transaction::records`), so a reopen writes them again.
//!   - No undo reaches a job's writes. So that an undo or redo of an earlier write of the same record cannot write
//!     over them, those earlier writes leave the undo and redo stacks.
//!   - Drafts for LP3's amendment list (PHS.52): the digest's form, the code for a record from another row, the
//!     history cut and the journal key `records`.
//! - The test hooks (`hold_commit_lane`, `fault_replace_head`, `store_mut`) exist only in builds with debug assertions
//!   or the `test-hooks` feature (L15).
//! - The per-launch secret comes from the standard library's random hasher keys, the clock and the process ID, hashed
//!   with SHA-256. A product build needs an operating-system random source.

#![forbid(unsafe_code)]

mod forms;
pub mod frame;
pub mod layer;
pub mod receipt;
pub mod shell;
pub mod store;
pub mod table;
pub mod token;
pub mod workspace;

pub use frame::{FrameChain, FrameStamp, HeadlessFrames, PresentedFrame, ShownPreview};
pub use layer::{Caller, CommandLayer, Invocation, JobOutput, Preview, PreviewState, ProgramJob, POLICY_ROW};
pub use receipt::{
    code, digest, ActionHint, CancelReason, CommitRecord, Completion, CompletionKind, Direction, Error, PreviewDeps,
    Receipt, ResultRef, Revision, Status,
};
pub use shell::{draft_args, word_draft, ArgSource, KeyBinding, Shell};
pub use store::{
    identity_from_json, identity_json, policy_from_name, policy_name, Fate, Genesis, History, MemoryStore, Reopen,
    Store, Transaction, TransactionKind,
};
pub use table::{Kind, Lane, Overlay, PreviewRule, Row, Table, UndoClass};
pub use token::{PreviewBinding, Secret, Token, TokenBinding};
pub use workspace::{Provenance, RecordRead, RecordValue, RecordWrite, ResolvedDraft, WorkspaceRecord, RECORD_ROW};

/// The sources of the command layer and its loop runner, with the stored command table, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 12] = [
    ("c600-command/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-command/src/bin/c600-lp3-oracle.rs", include_bytes!("bin/c600-lp3-oracle.rs")),
    ("c600-command/src/frame.rs", include_bytes!("frame.rs")),
    ("c600-command/src/layer.rs", include_bytes!("layer.rs")),
    ("c600-command/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-command/src/receipt.rs", include_bytes!("receipt.rs")),
    ("c600-command/src/shell.rs", include_bytes!("shell.rs")),
    ("c600-command/src/store.rs", include_bytes!("store.rs")),
    ("c600-command/src/table.rs", include_bytes!("table.rs")),
    ("c600-command/src/token.rs", include_bytes!("token.rs")),
    ("c600-command/src/workspace.rs", include_bytes!("workspace.rs")),
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
