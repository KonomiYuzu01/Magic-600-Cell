//! The research output panel's rows (LP5 plan, piece R10; section 6): the data the panel reads and the rows it invokes,
//! all through the command layer. The panel's look, layout, text and motion belong to ADR-D02 and K2; nothing here
//! draws.
//!
//! - [`rows`]: the eleven rows of section 6 as LP5's own block of the command table,
//!   `docs/progress/1.0/command-rows-research.json` (format `c600-command-rows-delta-1`; owner's LP5-1: ordinary
//!   additions after the LP3 freeze, CM.19 and CM.20, changing no frozen row). H-08 holds the block as its fifth delta
//!   (owner, 23 November 2026, item 3.1), so [`rows::table`] is H-08 with the command layer's draft rows and adds no
//!   row of the block a second time. [`rows::palette`] is KN5.6's availability table for the panel's controls, which a
//!   palette reads once per generation.
//! - [`host`]: the research host that serves the rows ([`Host::serve`], the command layer's `serve_row`) and answers
//!   each row's job from its own data: the files granted to it, the imports it published, the claim bindings of
//!   `c600-archive` and the check runs it made through R8's job adapter. No row is answered by running language code
//!   (KN5.5); the command layer gains no edge (ONE.48). [`Host::read`] gives the panel a row's answer with charter D2's
//!   state: current, pending, stale (naming the revision it was computed at), or an error (ONE.12).
//! - [`files`]: the host's file grant. `file` rows read and write only granted files (PS3.17, LB5.18).
//!
//! H-08's `session.branch.select` and `job.stop` stay as H-08 has them ([`rows::H08_ROWS`]): the panel invokes them,
//! and the research host serves neither. A running check or reproduction stops through the handle of
//! [`Host::stopper`], the job's cooperative cancellation (KN7.6).
//!
//! # The human-solve boundary
//!
//! The rows read and check. None chooses, ranks or applies a solving word: no row is a commit or app row, none takes a
//! user-action token or prepares a preview, claim groups are listed in the claim matrix's order and runs in the order
//! they ran, and no answer is sorted by a score. No row runs a library entry, so no searching entry is reached: every
//! searching entry stays a research tier candidate that a solving session refuses (PHS.7, BND.64). The host serves the
//! rows only in a session of the research profile (draft; below).
//!
//! # Drafts
//!
//! Each choice below is this module's and settles nothing in the chapters it names.
//!
//! - Row IDs, kinds, lanes and argument types are the section 6 drafts (owner's answer to item 1 pending). Queries run
//!   on the `none` lane, the job rows on the `program` lane. `research.check.run` and `research.run.reproduce` are
//!   workspace rows whose undo class is `none`: a run's record is a fact of what ran, which no undo removes (LB5.7).
//! - The serve refuses a session of another profile than `research` with `LG3:capability.grant_mismatch`: the panel
//!   imports and shows research archives, which hold the archive's own macros, so a solving session gets none of it.
//! - Each answer is `{row, revision, research, answer}`: `revision` is the command layer's revision the job read
//!   (KN0.2), and `research` the SHA-256 of the host's research data when the answer was made (its grant, imports, runs,
//!   reproductions and exports). An answer is current while the receipt's revision is the head and the research data is
//!   unchanged; otherwise it is stale, so a commit, a workspace write, an import or a check makes the earlier answers
//!   stale, and a query asked again answers at the head. The answer is the host's, and the receipt's result reference
//!   carries its digest, which [`Host::read`] checks.
//! - A check run is pinned at a stated input (`level`, `digest`, and the identity of `puzzle`: 600-cell-Full, or the
//!   jumbling identity of the check's `menu` input), as an archived certificate states its start. A check of a claim
//!   group first resolves the claim (LB5.14): the group's preflight must be `ready`, the checker one of the group's claim
//!   checkers and the certificate the group's record, or its `via` record. Only then does its verdict appear beside the
//!   group's preflight, and only its reports enter the group's evidence, under a key of the group's statement scoped to
//!   the run's pinned input (`crate::evidence::admit`).
//! - Records written, each under R8's provenance block: `research.run/<sha256 of the run record>` holds the run
//!   record's FP.5 text and its reports' FP.5 text, and `research.reproduction/<same>` DET.7's latest answer for it.
//! - A certificate or input of `research.check.run` names a granted member of a published import, stated by the
//!   corpus manifest's digest, or else a granted file, stated by its own bytes. `research.obligation.export`'s
//!   `statement` is a typed StatementId; its body is a granted file with that SHA-256, or none, which the translator
//!   refuses with `LB5:input.obligation_untranslatable`. A claim group's statement content names a record's claim by
//!   its pointers and is no body in the obligation vocabulary, so the host never passes it as one: the claim groups,
//!   the record-only anchors among them, have no typed body to export yet.
//! - A bundle carries each named record with the stored inputs its claim bindings name, a granted one with its bytes and
//!   any other as missing, and the bindings' missing inputs without a digest; its identity is the bindings' jumbling
//!   identity. The bytes are built in full before the granted file is written, so a refusal writes nothing.
//!
//! # Not built
//!
//! - The host keeps its imports, runs and exports in memory. After a reopen the workspace records hold every run and
//!   reproduction, but the host does not read them back yet, and it serves its rows again as every host does.
//! - `session.branch.select` and `job.stop` are the command layer's; it does not build `job.stop` today.
//!
//! Tests: `tests/panel.rs`, the headless driver, and `tests/obligation_export.rs`, the export row's checks. R10 has no
//! indexed conformance test of its own (LP5 plan, section 5); their markers name the parts they run of R8's and R9's
//! tests (CT-LB5-06, CT-LB5-11, CT-PS1-26, CT-RTE-23, CT-ERR-19, CT-DET-24).

/// The command layer's JSON value of a [`Json`](c600_session::persist::codec::Json), read by the kernel's strict JSON
/// reader (this crate has no edge of its own to the kernel, so it reaches that reader through `c600-checkers`).
macro_rules! kernel_value {
    ($json:expr) => {
        c600_checkers::claims::document(&c600_session::persist::codec::canonical(&$json)).map_err(|_| {
            c600_command::Error::new(c600_command::code::PERSISTENCE_FORMAT, "the value does not read as JSON")
        })
    };
}

mod answers;
pub mod files;
pub mod host;
pub mod rows;
mod wire;

pub use files::Grant;
pub use host::{Host, Read, D2};
pub use rows::{palette, table, PanelRow, DELTA, H08_ROWS, MODULE, ROWS};
pub use wire::invoke;
