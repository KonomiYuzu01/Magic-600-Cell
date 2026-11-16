//! LP3 C7's durable conformance cases, run on M6's session store (`c600-session`) and C4's module closures
//! (`c600-module`) through their public APIs. The crate has no library code: its tests are its content.
//!
//! KN5 gets one implementation, owned by LP2 under PHS.18 (integrator, 16 November 2026). The LP2 integrator placed
//! it in `c600-command`, which keeps its `Store` trait, with `c600-session` implementing the trait; LP2 writes it on
//! branch `claude/lp2-kn5-one`. Until it lands, the tests here drive `c600-session` as it is and change neither it
//! nor `c600-command`. Where a conformance part needs KN5 logic that M6 does not have yet, the test file names the
//! part as waiting for LP2's KN5 implementation instead of building it here.
//!
//! # Tests
//!
//! - `tests/ct_rte_08_persisted_word.rs`: CT-RTE-08's persisted-word case of BM2, its durable leg (BND.22, BND.84,
//!   BND.85, LG5.1, LG5.6). An approved own tool stores a word; the host persists it through M6's `workspace.set` with
//!   the boundary's provenance block; the session is closed and opened again; the word read back keeps the block, the
//!   block names the module the approval binds, preview admission refuses it, and a solving run that proposes it is
//!   refused at admission, with nothing reaching the engine.
//! - `tests/kn5_on_session.rs`: the parts of CT-EV3-07 (EV3.2, EV4.15: an explicit workspace command joins the durable
//!   action order; a view cannot write workspace state or make undo stale) and of CT-LB2-03's work-sheet part (LB7.2:
//!   reusing a named work sheet invalidates the old preview) that M6's public API can show.
//!
//! # Drafts
//!
//! - **The host's delivery of a stored word** (BND.22, KN5.15, ONE.77, LB7.2). M6 refuses every row but a job row from
//!   a program and has no path from a program's workspace write to the store, so a tool's write reaches M6 here as a
//!   `workspace.set` the shell delivers: the payload is the word's letters as a JSON list, the form M6's
//!   `operation.preview` reads, and the provenance is the boundary's block bytes, unchanged. C5's host bridge decides
//!   the real path.
//! - **Reading a record back** (PS1.16). M6 has no public reader of a workspace record, so the tests read the record
//!   from the journal's durable bytes through `c600_session::journal::scan` and M6's draft frame form. A
//!   `Session::record(name)` accessor would replace this.
//! - **Views on M6** (EV3.2). M6 has no view rows. The tests use what a view stands on in M6 instead: a query job, an
//!   opened snapshot and a preview, and a program's attempts to write the workspace, on the program's channel and from
//!   a job.
//!
//! # Waiting for LP2's KN5 implementation
//!
//! - KN5.26: `operation.preview` of a workspace record does not read the record's block or its parts through the
//!   BND.84 adapter, and M6 has no session profile, so M6's own preview admission of a persisted tool word is not
//!   tested here; `c600_run::boundary::admit_preview` on the read-back block stands in for it.
//! - LB7.2's work-sheet structure (named roles, re-resolution and a fresh exact analysis) and EV3.2's view rows.

#![forbid(unsafe_code)]
