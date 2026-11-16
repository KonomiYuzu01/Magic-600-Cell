//! The representations tool, headless half (LP3, C6): LB6.5's row "Representation, work-set and filter tools", class
//! `view_aid`, over LB4's layouts and atlases, map property checks and linked selection.
//!
//! - [`layout`]: sheets and atlases as display plans with their source maps, the layout filter that leaves the
//!   analysed set alone, and linked selection with its computation counter (LB4.4, EV3.10).
//! - [`chart`]: a survey of a chart over camera poses, exact verdicts at exact poses and display data everywhere
//!   (LB4.2), and the admission of a user chart as far as `c600-display` reaches it (LB4.3).
//! - [`tool`]: the tool's 600 Cell Script source, its manifest, the review of its class against the exact entry
//!   definitions and the row set, and the form of a review record (BND.37).
//! - [`rows`]: what each of the tool's six rows needs from the command layer, which refuses them as not built.
//!
//! The crate draws nothing and owns no device, renderer, clock or session. It builds on the public APIs of
//! `c600-display`, `c600-tools`, `c600-check` and `c600-command` and repeats none of their logic: picks are resolved
//! by `c600-display`'s host through the installed source map, exact chart checks are `c600-display`'s, and each
//! entry's admission is `c600_tools::tool::admit_built_in`. Every result of its tests is source or synthetic
//! evidence; none is Windows, DirectX, input, long-session or performance evidence.
//!
//! # Drafts
//!
//! Each choice below is this crate's own and settles nothing in the chapters it names.
//!
//! - LB6.5, BND.27: the tool's manifest is Rust data in [`tool::manifest`], because the IR draft and the syntax carry
//!   no manifest yet (LG5.2, PS1.15). It declares the one class `view_aid`.
//! - LB6.5: one module, `representations`, with five entries. `filter_counts` is a view; the other four are commands,
//!   because the solver invokes their rows with arguments. `set.save` has no entry: its `members` argument is a list,
//!   and `c600-ir-draft-1` has no list type.
//! - LG2.2, KN1.5: an entry takes a natural where its row takes a cell, a layer or a filter-preview reference, and
//!   text where its row takes one of two named values, because the IR draft has no identity or reference type. The
//!   snapshot queries and workspace records the entries name (`filter.preview`, `structure.compare`, `filter.apply`,
//!   `structure.layer`) are the rows' own IDs; no host answers them yet.
//! - BND.37, BND.63: the class review in [`tool::review_class`] reads `view_aid` as: no entry prepares a preview or
//!   calls what the checker cannot resolve; no entry requests a capability or has a file, randomness or budget effect;
//!   a workspace change sits in a command that serves a workspace row and reads no state-derived data, so that what
//!   it stores is what the solver named; every row is a query, view or workspace row of the stored command table
//!   that takes no user-action token and needs no commit preview. The review follows no value through an entry's
//!   body: "reads no state" is the checker's summary of the whole entry.
//! - BND.37, KN5: the row test in [`tool::row_within_view_aid`] reads a row's kind, token and preview rule and
//!   nothing else. The stored table has no field that says a query row prepares a commit preview, so
//!   `operation.preview` passes the row test, and an entry that proposes to it is found by the entry review instead.
//! - LP3 plan, C6: [`rows::NEEDS`] writes the completion kind, receipt and undo entry each of the six rows would
//!   need. They are requirements read from H-08's row fields and EV4.15, not a contract: the command layer decides
//!   them when it builds the rows.
//! - BND.37, CT-BND-15: an entry digest is the SHA-256 of a line naming this draft, the module, the entry and the
//!   SHA-256 of the module's source bytes ([`tool::entry_digest`]). A review record ([`tool::ReviewRecord`]) names the
//!   tool, the reviewer, the class, the source digest, every entry digest and the row set. BM5's review has not
//!   happened: the crate ships the record's form and its check, and no record.
//! - LB4.4: a representation is one display plan whose primitive 0 is its sheet or atlas and whose primitive 1 is a
//!   set overlay holding the mechanically analysed set with its exact-set digest. A layout filter rebuilds primitive
//!   0, the instance list and the source map's entries only. Filters of trees, complexes and graphs, and of layouts
//!   with edges or triangles, are not built.
//! - LB4, linked selection: a representation with no occurrence of the selected object answers
//!   [`layout::Linked::NoImage`], a case of its own beside the images. An unknown relation (EV3.9) is not modelled
//!   here: off-lattice relations wait for M5.
//! - EV3.10: a linked selection is computed for its inputs (the selected object, the target plan, its subject and
//!   base state, and the map's entries) and kept; [`layout::LinkedSelection::frame`] reads it and computes nothing.
//! - LB4.2: between exact poses a survey takes the display decision for each vertex as a caller-supplied function,
//!   whose default is the CPU reference's own exclusion neighbourhood. The repertoire is `c600-display`'s draft
//!   (RPI.34 [O]): the stereographic chart over convex hulls of exact vertices in Q(√5), with the properties domain,
//!   injectivity and section membership.
//! - LB4.3: a user chart is bound to its stage when one of the chart's stages names the stage description's digest as
//!   its definition ([`chart::admit_user_chart`]), and the stage must be a vertex stage, since a chart maps positions.
//!   The description stands for what the stage checker would read from a checked stage module; that checker, HLSL
//!   compilation and every device step are not built in `c600-display`.
//! - EV3.10, CT-EV3-06: the relation counted in this crate's tests is the linked selection above and, in
//!   `c600-view`, a view's recomputation counters. The relationship strip's typed relations are built nowhere yet.

#![forbid(unsafe_code)]

pub mod chart;
pub mod layout;
pub mod rows;
pub mod tool;
