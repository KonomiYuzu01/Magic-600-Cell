//! The built-in tools of 600 Cell Script and the LP3 source route (LP3, C5; `docs/progress/1.0/language-lp3-plan.md`,
//! section 7).
//!
//! - [`tool`] holds the built-in 600-cell-Full tool: its `.6cs` source in edition `6cs-draft-1` (syntax candidate A),
//!   its module manifest with its declared class (BND.27, BND.37), its admission in the solving profile (it prepares
//!   previews and reads no state: LG3.13, BND.39) and one run of it on `c600-run` through the run-time boundary, which
//!   gives the program the fixed acknowledgement (BND.42, BND.43).
//! - [`bridge`] is the runtime's host on `c600-command`: each proposal the boundary admits becomes one runtime
//!   invocation of `operation.preview` with the literal word as its absolute argument and no token.
//! - [`expand`] is the source route's recipe expansion: a case step's recipe to its literal word, through the kernel,
//!   outside the program and without a state read.
//! - [`case`] reads the frozen oracle cases O01 to O07.
//! - [`views`] builds the tool's views: the display contract's publication of a proposal and its preview, the content
//!   the headless view model installs for it, and the adapter's presentation report to the shell.
//! - The binary `c600-lp3-source` replays O01 to O07 on the source route: each step's proposal comes from a run of the
//!   tool, and the shell presents and commits the preview on the key route.
//!
//! The tool runs in-process; the runtime's own process (C3) is not on main yet.
//!
//! # Drafts
//!
//! Each choice freezes later with the IR or the command table.
//!
//! - **Entry.** `full_600_cell.preview_word(w: Word)` is an exported command (LG3.6) that proposes `w` as given. The IR
//!   draft has no list types and no word operations, so the tool proposes the word it is given; the library entries of
//!   chapter 03 (LB1 to LB4) wait for the IR.
//! - **Target.** The proposal's target text names the command-layer row the proposal reaches, `operation.preview`.
//! - **Result.** The entry returns `record { base, id }`, read with `receipt_base` and `receipt_id`: the two receipt
//!   fields a previewing program may read (BND.42). A receipt itself has no canonical output form.
//! - **Manifest and class.** The IR and the syntax carry no module manifest yet, so [`tool::Manifest`] is Rust data.
//!   The declared class is LB6.5's candidate for the word/recipe composer and full-effect inspector, `analysis` and
//!   `checking`; the rows are `operation.preview`. No `PrincipleRecord` and no review record exist (LB6.3, BM5), so
//!   the review of the class against the entry (CT-BND-15, CT-LB6-03) is not done.
//! - **Built-in admission.** `c600_check::admit` with `Profile::Solving` is the column of a solver's own, forked or
//!   imported tool, which prepares no preview until the joint setting, so it refuses this tool. [`tool::admit_built_in`]
//!   applies the built-in column of chapter 07 section 8 over the checker's report and its capability-separation
//!   facts: no checker diagnostic; never both a state read and a preview (LG3.13); a previewing entry is an exported
//!   command (LG3.6) that calls no named function and proposes only a `Word` parameter as given, unshadowed (a word the
//!   solver named). The built-in column's two exceptions are not built. It does not judge a reading entry's workspace
//!   writes against the class's rows (BND.57): the manifest's rows are its proposals' targets only, so an entry that
//!   reads state and writes a workspace record is admitted with no grant (the relay of that record into a proposal is
//!   refused at the proposal). The API `c600-check` would need: an author or column parameter on `admit`, or an
//!   `admit_built_in` of its own.
//! - **Provenance.** The blocks name a draft capability contract, pinned as `c600-tools-draft-1`, and the checker
//!   pin `c600-check-lp2-draft` over `c600-check`'s source. The word argument enters with a manual block: the solver
//!   named it. The producer is the tool's module digest and entry, author `built_in`, profile `solving`.
//! - **Limits.** One run gets [`tool::LIMITS`]; the run's charges are information only.
//! - **Expansion.** A recipe the kernel accepts expands to its letters. A recipe the kernel refuses that is exactly one
//!   word step of integer moves fitting `i32` passes unchanged, so the command layer refuses the literal word as it
//!   refuses the draft on the command route. Any other refused recipe has no literal word: the tool does not run, and
//!   the step's record is the kernel's refusal. In O07 that is steps 3 to 7 and 9 (a fractional move, an empty
//!   recipe, three malformed stars and a recipe whose second step is a malformed star); steps 0, 1, 2 and 8 pass.
//! - **Bridge.** The bridge passes a proposal's target text through as the row ID, so the command layer's own rules
//!   decide: it refuses a runtime invocation of a commit or app row and any runtime invocation carrying a token
//!   (ADR-L02, KN5.15). Its command IDs are `<run>/<n>`, `n` counting the bridge's invocations in the run; for a run
//!   whose one proposal the boundary admits, that is the boundary's proposal ID. A proposal's tariff is the word's
//!   length (LG2.17) with a result bound of [`bridge::RESULT_BOUND`] bytes until the command table prices the row. The
//!   answer the shell sees is a receipt `{base, id, status, job}`; the layer's refusal or failure is its error code.
//!   The acknowledgement's `base` is `<session>@<epoch>/<head>`.
//! - **Source route.** `c600-lp3-source` keeps `c600-lp3-oracle`'s session names, shell prefixes, trace form and
//!   candidate block (`c600-command-lp3-loop` with the command layer's build), so the two routes' traces compare byte
//!   for byte; its summary adds the route `source`, this crate's [`build`] and `host: in-process`. Each step's run is
//!   named `<case>-<index>`. The shell commits with `Ctrl+Shift+Enter`, the draft keymap's chord for
//!   `operation.execute` (H-08's `key_0_4`), because the keymap binds `Ctrl+Enter` to `operation.preview`, which on
//!   this route the program invokes; undo and redo are `Ctrl+Z` and `Ctrl+Y`.
//! - **Receipts file.** `--receipts` writes every receipt in order with its caller and, per step, the literal word's
//!   digest and length, the kernel's refusal of a recipe with no literal word, the preview's draft digest, the case
//!   recipe's draft digest, and the preview's result digest rebased onto the case recipe's draft. The preview-ready
//!   result digest covers the draft's digest, so it differs from the command route's for a recipe that is not one word.
//! - **Negative control.** `--tamper <case>:<index>` negates the first letter of that step's literal word before the
//!   tool runs.
//! - **Build string.** [`build`] covers this crate's sources and the command layer's build string, which covers the
//!   kernel's. `c600-run`, `c600-check`, `c600-syntax` and `c600-ir` have no build string yet, so it does not cover
//!   them.
//! - **Views are built by the host.** The tool prepares previews and reads no state (LQ21, LG3.13), and in a solving
//!   session only the shell sees a preview object (charter E6), so the tool publishes no view. [`views::proposal_view`]
//!   is host code: it builds the view from what the run left for the shell and from the engine's preview, and refuses a
//!   preview that another command built, that is of another word, or that reads another revision (EV3.1, BND.4).
//! - **Producing digest and ranges.** Every link carries the SHA-256 of the source bytes that ran, the manifest's
//!   `source_sha256`, and the bytes are pinned with each publication (RM.10). The ranges are the parser's source map
//!   of the checked program (LG4.12): the entry's declaration, each `propose` call (IR path order) and its word
//!   argument. [`views::Links::of`] parses and lowers the source again to read that map, because [`tool::Tool`] keeps
//!   the program and not its source map.
//! - **Canonical objects.** A view's instances are the orbits the word moves, `ObjectId::Orbit` of the preview
//!   effect's support rows (EV3.1, EV3.5): the Rust kernel names no grip, cell or piece for a letter, so the view
//!   claims none. The support plan shows them over the session state and links them to the `propose` call; the derived
//!   plan shows the same orbits again over the end state and links them to the word argument, so each orbit has two
//!   instances with one canonical ID. Instance IDs count from 1 through both plans; a plan's map has the plan's ID.
//! - **Nodes.** The entry is a container node whose text is `<module>.<entry>`; the proposal is a control node whose
//!   text is the word in literal form (`word[+1, -2]`), then a refusal's code, and whose state content is the SHA-256
//!   of the preview's canonical summary under the row `operation.preview`. No node is a relation and none carries a
//!   claim: the review of the class against the entry is not done (see "Manifest and class").
//! - **Derived display.** The end state of a preview is a derived subject at level `labels` whose lineage is the
//!   kernel derivation from the session state at the base by the preview's literal digest, with the base revision as
//!   comparison base (KN2.11, EV3.4). A preview without an end state (a protected orbit would move) has no derived
//!   plan.
//! - **Imported display.** [`views::imported_view`] states lineage `imported` with the import digest and the
//!   comparison base its caller gives, and nothing else (KN2.14, EV3.12). The Rust layers have no import row yet, so
//!   the import record is the caller's; the tool imports nothing.
//! - **Revisions.** [`views::revision_ref`] maps a command-layer revision to the display contract's: the head `e<n>`
//!   becomes `n`, the mode must be `jumble_off` and the identity block must be of the layer's model (KN0.2).
//!   [`views::model_revision`] gives the view model the same counters and the label digest's 32 bytes. A view depends
//!   on the state and the protection revision, the two a preview is fresh for (KN0.8).
//! - **View-model content.** `c600-view` installs opaque bytes and counts. [`views::content`] gives each node as the
//!   display contract's canonical metadata of a publication holding that node alone, each plan's payload as that of
//!   the plan with its map, and the pinned source length as `source_bytes`; a retained primitive counts no layout
//!   row. This stands in until one publication type serves both crates (RPI.10, RM.20).
//! - **Declared class.** The view model's `Declared` holds one class and the manifest declares two (BND.27, RPI.83),
//!   so the host states one of the manifest's classes when it opens the view; the tests state `checking`. The view's
//!   source is the built-in source named after the module.
//! - **Presentation report.** [`views::report_presented`] calls the shell's `present` only when the composed frame's
//!   presented report names the layer's pending preview, which `c600-display` gives only for a preview its built-in
//!   chain drew (RM.26, ONE.41, RPI.76). It adds no token logic. `CommandLayer::present` and `Shell::present` take a
//!   preview ID and no presented-frame identity, so the layer cannot itself refuse a report of a user-stage frame.

#![forbid(unsafe_code)]

pub mod bridge;
pub mod case;
pub mod expand;
pub mod tool;
pub mod views;

/// The sources of the built-in tools, the bridge and the source route's runner, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 9] = [
    ("c600-tools/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-tools/src/bin/c600-lp3-source.rs", include_bytes!("bin/c600-lp3-source.rs")),
    ("c600-tools/src/bridge.rs", include_bytes!("bridge.rs")),
    ("c600-tools/src/case.rs", include_bytes!("case.rs")),
    ("c600-tools/src/expand.rs", include_bytes!("expand.rs")),
    ("c600-tools/src/full_600_cell.6cs", include_bytes!("full_600_cell.6cs")),
    ("c600-tools/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-tools/src/tool.rs", include_bytes!("tool.rs")),
    ("c600-tools/src/views.rs", include_bytes!("views.rs")),
];

/// The build string: the crate version, the SHA-256 of a `sha256sum`-style listing of [`SOURCES`], and the command
/// layer's build string.
pub fn build() -> String {
    let mut listing = String::new();
    for (name, bytes) in SOURCES {
        listing.push_str(&format!("{}  {name}\n", c600_digest::sha256_hex(bytes)));
    }
    let sources = c600_digest::sha256_hex(listing.as_bytes());
    format!("c600-tools {} sources-sha256:{sources}; {}", env!("CARGO_PKG_VERSION"), c600_command::build())
}
