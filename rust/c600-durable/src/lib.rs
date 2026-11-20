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
//!   KN0.8), so the press prepared before commits nothing. That write keeps the word's part with the record
//!   (CT-BND-27; LP2's pull request 272), so after another power loss a host that keeps no block admits the record's
//!   preview.
//! - `tests/kn5_on_layer.rs`: CT-EV3-07 on the layer over the journal (EV3.2, EV4.15, KN5.20). A commit, the solver's
//!   `workspace.set` and a commit take places 1 to 3 of one `session.undo` order; a hint taken before a later action
//!   of either kind is superseded; and undo, then redo, walk the three actions in order across both kinds, each after a
//!   power loss. A program's `workspace.set` and `session.undo` are refused, and a view, a language overlay row of kind
//!   `view`, publishes from a program and from the shell without writing the journal or making the undo hint stale.
//! - `tests/kn5_on_session.rs`: the same CT-EV3-07 parts on M6's own `Session` API, and CT-LB2-03's work-sheet part
//!   (LB7.2: reusing a named work sheet invalidates the old preview) with the sheet as a record that M6 sets again,
//!   since M6 has no work-sheet rows.
//! - `tests/ct_lb2_03_work_sheet_reuse.rs`: CT-LB2-03's work-sheet sentence through the layer's `worksheet.save`,
//!   `worksheet.load` and `worksheet.compare` rows on the journal, with C5's reader installed (LB7.2, LB7.3, LB2,
//!   KN0.8, KN5.23). A sheet saved from a draft record comes back equal at a reuse in the same session, after a power
//!   loss on a memory disk and in a new session on a file-system disk. Each reuse makes a presented preview of the
//!   sheet stale (`KN5:stale.revision_changed`), so the press prepared before it commits nothing, and a fresh engine
//!   preview of the reused sheet commits its complete operation. A reuse after that commit places the piece roles again
//!   and shows the moved piece without retargeting the sheet, and its undo restores the sheet as saved. A comparison of
//!   a sheet with the current work, LB7.2's fresh exact analysis of the sheet, completes as a query whose result
//!   digests it, writes nothing to the journal and keeps a presented preview fresh: its analysis of the sheet's
//!   complete operation from the head gains nothing, loses the positions the operation unfinishes, as LB2 counts them,
//!   and ends where the operation committed inline ends, and a sheet saved from a record of letters reports that record
//!   as its current macro version. After a power loss the comparisons come back equal, and a later write of the record
//!   of letters makes the macro version changed while the sheet's own word is still the one analysed. Each refusal of
//!   the three rows that LP2's draft names comes back with the layer's code, queues no job and changes neither the
//!   revision, the records, the pending preview nor the journal's bytes: no reader, in a session reopened without one,
//!   where no row is available either (`KN5:input.row_not_built`), the runtime (`LG3:capability.not_granted`), a
//!   malformed argument (`KN5:input.argument_form`), an unknown or removed record, a record that holds no operation to
//!   save, no saved sheet to reuse or compare, or no work to compare with, a malformed sheet among them
//!   (`KN2:input.named_object_invalid`), a sheet named at another version (`KN5:stale.revision_changed`) and a block
//!   that C5's reader refuses (`PS1:capability.provenance_incompatible`). Not run here: the refusals with Jumble on
//!   (`KN5:input.retained_tool_needs_jumble_off`, LG2.8), which `c600-command`'s `tests/jumble_on.rs` runs for the save
//!   and the reuse; entering Jumble on needs a twist menu pinned from `c600-jumble`, which is no dependency of this
//!   crate. A comparison while the exclusive lane is held (`KN7:busy.lane_full`) runs in `c600-command`'s
//!   `tests/worksheet.rs`.
//!
//! # Whether CT-LB2-03 runs whole
//!
//! It does not. Its first sentence, recipe chunking and the refusal of an oversized recipe, runs in `c600-kernel`'s
//! `tests/kernel.rs` (the kernel half) and in `c600-tools`' `tests/net_oversize.rs` and `tests/process.rs` (the command
//! and source routes), and C5 records chunked transport on the source route for a recipe with a star step, or a word
//! step with another field, as not built. Its second sentence, named work-sheet reuse invalidates the old preview, with
//! LB7.2's fresh exact analysis of a reused sheet (H-08's `worksheet.compare`, a query row with a job, with LB2's gains
//! and losses, and macro versions; LP2's pull request 277), runs through the layer's rows on the layer's memory store
//! (`c600-command`'s `tests/worksheet.rs`) and on M6's journal (`tests/ct_lb2_03_work_sheet_reuse.rs`). The layer does
//! not build reading a position role's occupant again; this crate builds no KN5 logic, so that waits for LP2.
//!
//! # Drafts
//!
//! - **The host's delivery of a stored word** (BND.22, KN5.15, ONE.77, LB7.2). C5's bridge grants a program no
//!   workspace write, and the layer refuses `workspace.set` to the runtime, so a tool's stored word reaches the journal
//!   as a `workspace.set` the solver's shell sends: the payload is the word's letters as a JSON list, the form a record
//!   draft reads, and the provenance is the boundary's block as text, unchanged. C5's runner writes no record itself.
//! - **The host's reader in these tests** (PS1.16, KN5.26). The tests install C5's reader under the fixture's policy
//!   pins, the pins their programs are approved under, rather than with `c600_tools::workspace::install`, which uses
//!   the pins of C5's own tools. A reopen restores the records and the part blocks the layer kept with their writes,
//!   but not the blocks the reader keeps: the host keeps a run's blocks again with `Reader::keep_run`, and a part that
//!   neither keeps reads unknown.
//! - **A built-in tool's stored word** (BND.84). Preview admission refuses a solver tool's word, so the fresh and stale
//!   preview of a persisted word use the same entry approved as a built-in tool in the tool-authoring profile. Its
//!   block lists the run's input block as its part.
//! - **Views on the layer** (EV3.2, KN5.3). The layer leaves `view.open` and `view.close` to the host's view model,
//!   which these tests do not install yet, so a view is a language overlay row of kind `view`, `tool.c7.panel`, whose
//!   program job the test finishes with a fixed value where the host would evaluate the view.
//! - **Views on M6** (EV3.2). M6 has no view rows. `tests/kn5_on_session.rs` uses what a view stands on in M6 instead:
//!   a query job, an opened snapshot and a preview, and a program's attempts to write the workspace, on the program's
//!   channel and from a job.
//! - **A block that C5's reader refuses, on the journal** (PS1.16, LB7.3). C5's reader reads only canonical FP.17
//!   blocks, so no record written through it holds another block. `tests/ct_lb2_03_work_sheet_reuse.rs` writes such a
//!   record, and a sheet saved from it, under a test reader that reads every block, standing in for a host's reader
//!   that accepted the block. After a power loss the journal replays both without a reader check, the sheet keeps the
//!   block, and C5's reader refuses the save, the reuse and the comparison with its error.
//!
//! # Waiting for LP2's KN5 implementation
//!
//! - KN5.26's session profile. Built: the layer reads a record draft's block through the host's reader and admits or
//!   refuses the record's word with the reader's answer and the part blocks the layer keeps, which
//!   `tests/ct_rte_08_persisted_word.rs` runs, and KN5.26's admission matrix (L1, L2), which `c600-command`'s
//!   `tests/kn5_admission.rs` runs (LP2's pull requests 270 and 272). Not built: a session profile, which the layer
//!   does not know, so the host's reader applies a solving session's admission in every session.
//! - EV3.2's view rows. LP2's part is built (pull request 271): the layer admits `view.open` and `view.close` and
//!   leaves them to the host's view model (`c600_command::Views`). No test here installs one yet, so both rows answer
//!   `KN5:input.row_not_built` and CT-EV3-07's view part runs on a language overlay row of kind `view`; LP3's next
//!   step serves `Views` with `c600-view`'s view model.
//! - Reading a position role's occupant again at a reuse or a comparison, which the layer does not build. The rest of
//!   the work-sheet structure, the named roles and their placement again, and LB7.2's fresh exact analysis are built,
//!   and `tests/ct_lb2_03_work_sheet_reuse.rs` runs them.

#![forbid(unsafe_code)]
