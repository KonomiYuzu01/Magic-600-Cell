//! The representations tool, headless half (LP3, C6): LB6.5's row "Representation, work-set and filter tools", class
//! `view_aid`, over LB4's layouts and atlases, map property checks and linked selection.
//!
//! - [`layout`]: sheets and atlases as display plans with their source maps, the layout filter that leaves the
//!   analysed set alone, and linked selection with its computation counter (LB4.4, EV3.10).
//! - [`chart`]: a survey of a chart over camera poses, exact verdicts at exact poses and display data everywhere
//!   (LB4.2), and the admission of a user chart as far as `c600-display` reaches it (LB4.3).
//! - [`tool`]: the tool's 600 Cell Script source, its manifest, the review of its class against the exact entry
//!   definitions and the row set, and the form of a review record (BND.37).
//! - [`rows`]: what each of the tool's six rows needs from the command layer, what the layer meets of it and what
//!   remains. A host serves five of the rows with the tool's entries; the layer serves `filter.apply` too (LP2's pull
//!   request 256), and no host runs its entry yet.
//!
//! The crate draws nothing and owns no device, renderer, clock or session. It builds on the public APIs of
//! `c600-display`, `c600-tools`, `c600-check` and `c600-command` and repeats none of their logic: picks are resolved
//! by `c600-display`'s host through the installed source map, exact chart checks are `c600-display`'s, and each
//! entry's admission is `c600_check::admit_built_in` with the class rows of [`tool::class_rows`]. Every result of its
//! tests is source or synthetic evidence; none is Windows, DirectX, input, long-session or performance evidence.
//!
//! # Drafts
//!
//! Each choice below is this crate's own and settles nothing in the chapters it names.
//!
//! - LB6.5, BND.27: the tool's manifest is Rust data in [`tool::manifest`], because the IR draft and the syntax carry
//!   no manifest yet (LG5.2, PS1.15). It declares the one class `view_aid`.
//! - LB6.5: one module, `representations`, with six entries, one for each row. `filter_counts` is a view; the other
//!   five are commands, because the solver invokes their rows with arguments. `set.save`'s entry takes the members as
//!   `List<Nat>`, the list type the IR draft gained in LP2 (pull request 176).
//! - LG2.2, KN1.5: an entry takes a natural where its row takes a cell, a layer or a filter-preview reference, a list
//!   of naturals where its row takes canonical object references, and text where its row takes one of two named
//!   values, because the IR draft has no identity or reference type. The snapshot queries and workspace records the
//!   entries name (`filter.preview`, `structure.compare`, `filter.apply`, `set.save`, `structure.layer`) are the rows'
//!   own IDs. A served row's records reach the command layer as its job's output; no host row answers the two
//!   snapshot queries from a kernel row yet (LP2's structure rows are in the layer since pull request 262).
//! - BND.37, BND.63: the class review in [`tool::review_class`] reads `view_aid` as: no entry prepares a preview or
//!   calls what the checker cannot resolve; no entry requests a capability or has a file, randomness or budget effect;
//!   a workspace change sits in a command that serves a workspace row and reads no state-derived data, so that what
//!   it stores is what the solver named; every row is a query, view or workspace row of the stored command table
//!   that takes no user-action token and needs no commit preview. The review follows no value through an entry's
//!   body: "reads no state" is the checker's summary of the whole entry.
//! - BND.57: BND.57's rows by class do not exist as data, so the class lists the grant-table rows the shipped entries
//!   use ([`tool::class_rows`]): the snapshot reads `filter.preview` and `structure.compare`, a published view, and the
//!   workspace records `filter.apply`, `set.save` and `structure.layer`. The built-in column refuses an entry that
//!   uses another row, and it refuses a store of a word the solver did not name at the write (LP2's column, pull
//!   request 170).
//! - BND.37, KN5: the row test in [`tool::row_within_view_aid`] reads a row's kind, token and preview rule and
//!   nothing else. The stored table has no field that says a query row prepares a commit preview, so
//!   `operation.preview` passes the row test, and an entry that proposes to it is found by the entry review instead.
//! - LP3 plan, C6: [`rows::NEEDS`] writes the completion kind, receipt and undo entry each of the six rows needs, what
//!   the command layer meets of them and what remains. They are requirements read from H-08's row fields and EV4.15,
//!   not a contract: the command layer decides them as it builds the rows. Its serving of the rows is LP2's draft
//!   (pull requests 200, 202, 203 and 256).
//! - LP3 plan, C6 and C5's item 1: the host that serves the five rows is a test host (`tests/served_rows.rs`), because
//!   a host in this crate would make `c600-run` a dependency of the library; the tests take it as a dev-dependency.
//!   It runs an entry in process through `c600-run`'s boundary, maps a job's arguments to the entry's parameters by
//!   name, answers the two snapshot reads with the state number of the job's revision, and fails a running program's
//!   job, whose arguments carry no provenance block (BND.22). The source route runs the same entry in process on the
//!   same arguments at the same revision without the layer (not C5's runner `c600-lp3-source` or the runtime's own
//!   process), and the two routes give the same value, views and records.
//! - BND.37, CT-BND-15: an entry digest is the SHA-256 of a line naming this draft, the module, the entry and the
//!   SHA-256 of the module's source bytes ([`tool::entry_digest`]). A review record ([`tool::ReviewRecord`]) names the
//!   tool, the reviewer, the class, the source digest, every entry digest and the row set. BM5's review has not
//!   happened: the crate ships the record's form and its check, and no record.
//! - BND.37, CT-LB6-03: the class review covers every exported function of the tool's module, so an export the
//!   manifest does not list is a finding; [`tool::review_source`] reviews source bytes only under the manifest that
//!   names their SHA-256, and finds bytes that do not lower. A blank reviewer name is no reviewer.
//! - LB4.4: a representation is one display plan whose primitive 0 is its sheet or atlas and whose primitive 1 is a
//!   set overlay holding the mechanically analysed set with its exact-set digest. A layout filter rebuilds primitive
//!   0, the instance list and the source map's entries only. Filters of trees, complexes and graphs, and of layouts
//!   with edges or triangles, are not built.
//! - LB4, linked selection: a representation with no occurrence of the selected object answers
//!   [`layout::Linked::NoImage`], a case of its own beside the images. An unknown relation (EV3.9) is not modelled
//!   here; `c600_jumble_tool::relation` has one at pose level, a slot's current occupant, with "none: off the
//!   lattice" and `unknown` as two values (C8).
//! - EV3.10: a linked selection is computed for its inputs (the selected object, the target plan, its subject and
//!   base state, and the map's entries) and kept; [`layout::LinkedSelection::frame`] reads it and computes nothing.
//! - LB4.2: between exact poses a survey takes the display decision for each vertex as a caller-supplied function,
//!   whose default is the CPU reference's own exclusion neighbourhood. The repertoire is `c600-display`'s draft
//!   (RPI.34 [O]): stereographic and projective charts over unions of cells with exact vertices in Q(√5), with the
//!   properties domain, injectivity, embedding, isometry, homomorphism and section membership. A survey checks the
//!   stereographic chart only, the chart the CPU reference evaluates.
//! - LB4.3: a user chart is bound to its stage when one of the chart's stages names the stage description's digest as
//!   its definition ([`chart::admit_user_chart`]), and the stage must be a vertex stage, since a chart maps positions.
//!   The description stands for what the stage checker would read from a checked stage module; that checker, HLSL
//!   compilation and every device step are not built in `c600-display`.
//! - EV3.10, CT-EV3-06: the relation counted in this crate's tests is the linked selection above and, in
//!   `c600-view`, a view's recomputation counters. Of the relationship strip's typed relations, only a slot's current
//!   occupant at pose level is built, in `c600_jumble_tool::relation` (C8); the strip's data contract (chapter 06,
//!   RPI.65) is built nowhere yet.
//!
//! # Not built
//!
//! What the rows still need, with the API a host would use. C6 changed nothing in `c600-command` or the kernel. Of the
//! items below, LP2 has built the command layer's part of `filter.apply`, of a running program's job and of the two
//! snapshot reads; what is left of them is the host's, which is LP3's. LP2 has also built the `CanonicalObjectRefs`
//! form (pull request 269), so the layer refuses a member that is no piece or position at admission. [`rows::NEEDS`]
//! gives the same per row.
//!
//! - `filter.apply`: the layer serves the row (LP2's pull request 256: the gate `fresh_filter_preview`, the `filter`
//!   preview rule, the `FilterRef` and `FilterPreviewRef` forms and a workspace inverse). `tests/served_rows.rs` only
//!   serves it and checks the refusal of a stale filter preview. Next: the entry `apply_filter` takes the layer's
//!   `FilterPreviewRef` in place of a natural, and the row runs through both routes.
//! - A running program's job: LP2's `CommandLayer::invoke_with_inputs` (pull request 264) takes the provenance blocks
//!   of an invocation's arguments, and `ProgramJob::inputs` holds them. The host builds them from the run's context.
//!   The test host builds none yet, so it still fails a running program's query or view job. Next: the host builds
//!   them.
//! - Kernel rows for the two snapshot reads: LP2's structure rows answer them (pull request 262:
//!   `CommandLayer::compare_cells`, `cell_layers` and `layer_counts`, which read no state and refuse with Jumble on).
//!   Next: entries that pass their arguments with `read_snapshot<T>(query, x)` (pull request 199), and a host row in
//!   the tool's bridge. The entries read a literal query with no argument, and the test host answers it with the
//!   state number.
//! - `set.save`: a record for each saved set, where the entry writes the one record `set.save`, and the saved set's
//!   exact-set digest as the result reference, where the layer gives the output's digest.
//! - `view.effect.scope`: a display that applies the published scope.

#![forbid(unsafe_code)]

pub mod chart;
pub mod layout;
pub mod rows;
pub mod tool;
