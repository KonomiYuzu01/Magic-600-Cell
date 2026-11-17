//! LP3 C7's durable conformance cases, run through public APIs on KN5's command layer over M6's journal
//! (`c600-command`'s layer on `c600-session`'s `JournalStore`), on M6's session store and on C4's module closures
//! (`c600-module`). The crate has no library code: its tests are its content.
//!
//! KN5 gets one implementation, owned by LP2 under PHS.18 (integrator, 16 November 2026): `c600-command`, whose `Store`
//! trait `c600-session` implements. Its stages 1 to 5 are on main (LP2's pull requests 174 and 194). The tests here
//! drive it with LP3's host pieces, C5's provenance reader among them (`c600_tools::workspace`), and change neither it
//! nor `c600-session`. Where a conformance part needs KN5 logic that the layer does not have yet, the test file names
//! the part as waiting for LP2's KN5 implementation instead of building it here.
//!
//! # Tests
//!
//! - `tests/ct_rte_08_persisted_word.rs`: CT-RTE-08's persisted-word case of BM2, its durable leg (BND.22, BND.84,
//!   BND.85, LG5.1, LG5.6), on the layer over the journal with C5's reader installed. An approved own tool stores a
//!   word, which a program cannot deliver, and the solver writes it with the boundary's provenance block through
//!   `workspace.set`. After a power loss on a memory disk, and in a new session on a file-system disk, the record and
//!   its receipt come back equal (KN5.23); the block names the module the approval binds and no other; the layer's
//!   preview of the record as a `{"record": name}` draft is refused with the reader's BND.84 refusal and changes
//!   nothing (KN5.26); and a solving run that proposes the word is refused at admission, with nothing reaching the
//!   engine. The same word stored by the entry approved as a built-in tool fails closed on its missing part (BND.85)
//!   until the host keeps the run's blocks again. Then its record draft is previewed and presented, a write of another
//!   record keeps the preview fresh, and a later write of that record makes it stale (`KN5:stale.revision_changed`,
//!   KN0.8), so the press prepared before commits nothing.
//! - `tests/kn5_on_layer.rs`: CT-EV3-07 on the layer over the journal (EV3.2, EV4.15, KN5.20). A commit, the solver's
//!   `workspace.set` and a commit take places 1 to 3 of one `session.undo` order; a hint taken before a later action
//!   of either kind is superseded; and undo, then redo, walk the three actions in order across both kinds, each after a
//!   power loss. A program's `workspace.set` and `session.undo` are refused, and a view, a language overlay row of kind
//!   `view`, publishes from a program and from the shell without writing the journal or making the undo hint stale.
//! - `tests/kn5_on_session.rs`: the same CT-EV3-07 parts on M6's own `Session` API, and CT-LB2-03's work-sheet part
//!   (LB7.2: reusing a named work sheet invalidates the old preview), which runs on M6 only.
//!
//! # Drafts
//!
//! - **The host's delivery of a stored word** (BND.22, KN5.15, ONE.77, LB7.2). C5's bridge grants a program no
//!   workspace write, and the layer refuses `workspace.set` to the runtime, so a tool's stored word reaches the journal
//!   as a `workspace.set` the solver's shell sends: the payload is the word's letters as a JSON list, the form a record
//!   draft reads, and the provenance is the boundary's block as text, unchanged. C5's runner writes no record itself.
//! - **The host's reader in these tests** (PS1.16, KN5.26). The tests install C5's reader under the fixture's policy
//!   pins, the pins their programs are approved under, rather than with `c600_tools::workspace::install`, which uses
//!   the pins of C5's own tools. A reopen restores the records but not the blocks the reader keeps: the host keeps a
//!   run's blocks again with `Reader::keep_run`.
//! - **A built-in tool's stored word** (BND.84). Preview admission refuses a solver tool's word, so the fresh and stale
//!   preview of a persisted word use the same entry approved as a built-in tool in the tool-authoring profile. Its
//!   block lists the run's input block as its part.
//! - **Views on the layer** (EV3.2, KN5.3). The layer builds none of H-08's `view` rows, so a view is a language
//!   overlay row of kind `view`, `tool.c7.panel`, whose program job the test finishes with a fixed value where the
//!   host would evaluate the view.
//! - **Views on M6** (EV3.2). M6 has no view rows. `tests/kn5_on_session.rs` uses what a view stands on in M6 instead:
//!   a query job, an opened snapshot and a preview, and a program's attempts to write the workspace, on the program's
//!   channel and from a job.
//!
//! # Waiting for LP2's KN5 implementation
//!
//! - KN5.26, partly built. Built: the layer reads a record draft's block through the host's reader and admits or
//!   refuses the record's word with the reader's answer, which `tests/ct_rte_08_persisted_word.rs` runs. Not built:
//!   the rest of KN5.26's admission matrix (L1, L2); a session profile, which the layer does not know, so the host's
//!   reader applies a solving session's admission in every session; and a durable home for a word's part blocks. M6's
//!   journal keeps a record's own block only, so after a reopen a part is found only among the blocks the host keeps
//!   again, and CT-BND-27's recursive parts are not run.
//! - EV3.2's view rows. The layer refuses H-08's `view` rows, `view.open` and `view.close` among them, with
//!   `KN5:input.row_not_built`, so CT-EV3-07's view part runs on a language overlay row of kind `view`.
//! - LB7.2's work-sheet structure (named roles, re-resolution and a fresh exact analysis).

#![forbid(unsafe_code)]
