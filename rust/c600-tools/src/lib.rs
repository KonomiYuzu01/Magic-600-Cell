//! The built-in tools of 600 Cell Script and the LP3 source route (LP3, C5; `docs/progress/1.0/language-lp3-plan.md`,
//! section 7).
//!
//! - [`tool`] holds the built-in 600-cell-Full tool: its `.6cs` source in edition `6cs-draft-1` (syntax candidate A),
//!   its module manifest with its declared class (BND.27, BND.37), its admission in the solving profile (it prepares
//!   previews and reads no state: LG3.13, BND.39) and one run of it on `c600-run` through the run-time boundary, which
//!   gives the program the fixed acknowledgement (BND.42, BND.43).
//! - [`bridge`] is the runtime's host on `c600-command`: each proposal the boundary admits becomes one runtime
//!   invocation of `operation.preview` with the literal word as its absolute argument and no token, in one word step
//!   or, for the input recipe's literal word longer than one chunk, in the solver's word steps.
//! - [`expand`] is the source route's recipe expansion: a case step's recipe to its literal word, through the kernel,
//!   outside the program and without a state read.
//! - [`case`] reads the frozen oracle cases O01 to O07.
//! - [`views`] builds the tool's views: the display contract's publication of a proposal and its preview, the content
//!   the headless view model installs for it, and the adapter's presentation report to the shell.
//! - [`workspace`] is the host's provenance reader on the command layer (KN5's stage 3): it reads a workspace record's
//!   block before `workspace.set` writes the record (PS1.16), and admits or refuses a draft that names a record
//!   (KN5.26, BND.84, BND.85).
//! - [`supports`] reads the 1,200 generators' complete supports from the kernel and builds LB1.10's independence
//!   relation `full-disjoint-support/1`, which a host pins for a run's trace keys (LP2 draft).
//! - The binary `c600-lp3-source` replays O01 to O07 on the source route: each step's proposal comes from a run of the
//!   tool, and the shell presents and commits the preview on the key route.
//!
//! The tool runs on either of two hosts: in process, on `c600-run` inside the caller's process, or on the process
//! host ([`process`]), in the child process of `c600-runtime` (C3; route RT1, RTE.21). On both, the same run-time
//! boundary and the same bridge answer the program in the caller's process, and every record names the host.
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
//!   imported tool, which prepares no preview until the joint setting, so it refuses this tool. [`tool::Tool::new`]
//!   admits the entry with `c600_check::admit_built_in`, the built-in column of chapter 07 section 8 that LP2 added
//!   to the checker (pull request 170) when this crate asked for it; this crate keeps no admission rule of its own.
//!   Draft: BND.57's rows by class do not exist as data, so [`tool::class_rows`] lists the prepare-a-preview row of
//!   each of the manifest's rows, `operation.preview`, and nothing else. An entry that also reads state, publishes a
//!   view or writes the workspace therefore uses a row its class does not list and is refused, and a store of a word
//!   the solver did not name is refused at the write as well as its relay at the proposal. The built-in column's two
//!   exceptions are not built.
//! - **Provenance.** The blocks name a draft capability contract, pinned as `c600-tools-draft-1`, and the checker
//!   pin `c600-check-lp2-draft` over `c600-check`'s source. The word argument enters with a manual block: the solver
//!   named it. The producer is the tool's module digest and entry, author `built_in`, profile `solving`.
//! - **Limits.** One run gets [`tool::LIMITS`]; the run's charges are information only.
//! - **Expansion.** A recipe the kernel accepts expands to its letters. A recipe the kernel refuses that is exactly one
//!   word step of integer moves fitting `i32` passes unchanged as the tool's argument: a word with a letter outside 1
//!   to 1200 in magnitude is refused at the run's start (see "Words with no transfer encoding"), and the command layer
//!   refuses any other as it refuses the draft on the command route. Any other refused recipe has no literal word:
//!   the tool does not run, and the step's record is the kernel's refusal. In O07 that is steps 3 to 7 and 9 (a
//!   fractional move, an empty recipe, three malformed stars and a recipe whose second step is a malformed star);
//!   steps 0, 1, 2 and 8 pass.
//! - **Bridge.** The bridge passes a proposal's target text through as the row ID, so the command layer's own rules
//!   decide: it refuses a runtime invocation of a commit or app row and any runtime invocation carrying a token
//!   (ADR-L02, KN5.15). Its command IDs are `<run>/<n>`, `n` counting the bridge's invocations in the run; for a run
//!   whose one proposal the boundary admits, that is the boundary's proposal ID. A proposal's tariff is the word's
//!   length (LG2.17) with a result bound of [`bridge::RESULT_BOUND`] bytes until the command table prices the row. The
//!   answer the shell sees is a receipt `{base, id, status, job}`; the layer's refusal or failure is its error code.
//!   The acknowledgement's `base` is `<session>@<epoch>/<head>`.
//! - **Source route.** `c600-lp3-source` keeps `c600-lp3-oracle`'s session names, shell prefixes, trace form and
//!   candidate block (`c600-command-lp3-loop` with the command layer's build), so the two routes' traces compare byte
//!   for byte; its summary adds the route `source`, this crate's [`build`] and the host. Each step's run is
//!   named `<case>-<index>`. The shell commits with `Ctrl+Shift+Enter`, the draft keymap's chord for
//!   `operation.execute` (H-08's `key_0_4`), because the keymap binds `Ctrl+Enter` to `operation.preview`, which on
//!   this route the program invokes; undo and redo are `Ctrl+Z` and `Ctrl+Y`.
//! - **Receipts file.** `--receipts` writes every receipt in order with its caller and, per step, the literal word's
//!   digest and length, the kernel's refusal of a recipe with no literal word, the preview's draft digest, the case
//!   recipe's draft digest, and the preview's result digest rebased onto the case recipe's draft. The preview-ready
//!   result digest covers the draft's digest, so it differs from the command route's for a recipe that is not one word,
//!   unless the bridge sent the recipe's own word steps: that draft is the case recipe's draft (see "Oversize and
//!   chunking").
//! - **Negative control.** `--tamper <case>:<index>` negates the first letter of that step's literal word before the
//!   tool runs. The runner takes the solver's word steps before it, so a tampered word is not the recipe's literal
//!   word and goes in one word step.
//! - **Build string.** [`build`] covers this crate's sources and the command layer's build string, which covers the
//!   kernel's. `c600-run`, `c600-check`, `c600-syntax`, `c600-ir` and `c600-runtime` have no build string yet, so it
//!   does not cover them.
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
//! - **Presentation report.** [`views::report_presented`] reports each composed frame to the layer through the shell as
//!   the layer's `PresentedFrame` ([`views::presented_frame`], [`views::frame_stamp`]): the stamp's device epoch,
//!   surface, surface generation and sequence, the preview the presented report names, which `c600-display` gives only
//!   for a preview its built-in chain drew, and a user-stage chain when the frame holds an extra preview display
//!   (RM.26, ONE.41, RPI.76). The layer refuses a frame that shows no preview, another preview or a preview drawn
//!   through a user stage, and a word commit's token binds the stamp of the frame that presented its preview. It adds
//!   no token logic. With no preview shown, pending or bound it reports nothing.
//! - **Process host** (RTE.21, ONE.69, LQ9; KN5.15; BND.42). [`process::ProcessHost`] sends one job per run to
//!   `c600-runtime`'s child: the source bytes, the manifest's module and entry, the literal word, [`tool::LIMITS`],
//!   the test tariffs, no grant and no source ranges, so the run's summary is the in-process host's. The boundary and
//!   the bridge stay in the caller's process as the `Host` of `c600_runtime::Runtime::run`: the child holds the
//!   program and its word, and no token, secret or command layer. One child serves every run of a host; after a
//!   fault the next run starts a fresh one (KN7.13). A run has no wall-clock deadline unless the caller sets one
//!   (KN7.10, DET.17).
//! - **Child admission** (ADR-L02, LG3.13, chapter 07 section 8). The child admits a job with `c600_check::admit`,
//!   which has no built-in column, and its solving column refuses every previewing program. So the job's profile is
//!   tool authoring, the built-in column stays the caller's ([`tool::Tool::new`]), and the process host sends only
//!   the source whose digest is the manifest's. The child's own admission of a built-in tool waits for the API named
//!   under "Built-in admission", and for a field in `c600-runtime`'s job options to carry it.
//! - **Words with no transfer encoding** (`PS1:input.noncanonical_value`; `c600-runtime`'s draft 7). A word with a
//!   letter outside 1 to 1200 in magnitude cannot cross to the child, so on the process host the parent refuses the
//!   job at the start stage, and in process [`process::run_in_process`] refuses the run there the same way, before
//!   the tool runs. (`Loaded::run` itself refuses only a letter of magnitude above 1200: a letter of 0 is a grip or
//!   item index in `c600-jumble-tool`'s reading of a `Word`.) On both hosts nothing is proposed. The runner takes the
//!   start-stage refusal as an `invalid-input` step only for a recipe the kernel already refused, and keeps the
//!   kernel's refusal in the step record. In O07 that is steps 0, 1, 2 and 8, which have no receipt on either host;
//!   the hosts' traces and records are the same beyond the host's name.
//! - **Host in the records** (plan section 7). `--host in-process|process`, in-process by default; `--runtime` names
//!   the runtime's binary, by default `c600-runtime` beside the runner (packaging decides later). The summary has
//!   `host` and `runtime_spawns`, each case of the summary `start_refused`, each case of the receipts file `host`,
//!   and each step record `host` (null when no word was given to a host) and `start_refusal`.
//! - **Oversize and chunking** (LB2.3, LB2.4, LB1.11). The bridge never chooses where to cut a word. The command
//!   route's chunking is the 0.4 recipe's word steps, and the source route carries the solver's own: for one run the
//!   host gives the bridge the input recipe's literal word and the length of each of its word steps
//!   ([`bridge::Steps`]), for a recipe of plain word steps only. A proposal longer than one chunk of 100,000 letters
//!   ([`bridge::WORD_CHUNK`]) that is exactly that literal word, letter for letter, goes in the solver's word steps,
//!   in order, in the one invocation ([`bridge::proposal_args`]). Every other proposal is one word step, as before:
//!   a word of at most one chunk, whatever the solver's steps, so O01 to O07 send the invocations they sent before;
//!   and a word that is not the input literal, which the layer refuses whole when it is longer than one chunk, as it
//!   refuses the same draft on the command route. The tariff stays the literal word's length, so the total bound of
//!   3,000,000 is unchanged, and one proposal stays one invocation, one preview job and one commit. Only a plain word
//!   step, exactly `{"kind": "word", "moves": [...]}`, counts ([`expand::word_steps`]), so the steps the bridge sends
//!   are the recipe's own, byte for byte. Not built: a recipe with a star step, or with a word step that has another
//!   field, keeps the single-step transport, so its literal word longer than one chunk is refused whole on the
//!   source route.
//! - **Net and Strict checks** (LB2.3, CT-LB2-02's Net and Strict halves). The tests take the Net check to be the
//!   layer's preview verdict at label level over a session's protected set, here the orbits one generator moves, and
//!   the commit's refusal when that verdict is not clear. The Strict check is the same after the shell sets the
//!   session's policy to Strict with `protection.policy.set` (LP2's pull request 198; the runtime may not invoke the
//!   row): the layer judges every executed literal prefix with the kernel's `strict_protection`, the preview's
//!   summary names the mode and the first refused prefix, and the commit is refused while a prefix moves a protected
//!   orbit. Not built: the captured lock and "an incomplete prefix check never returns safe" (LB2.3's coverage report
//!   and `LB2:protection.incomplete_coverage`), which are LP3's library rows over kernel rows and need a snapshot read
//!   with an argument that no host answers yet.
//! - **Inspector fixtures** (LB3.5, CT-LB3-07's checker half). No LB3 entry exists in the IR, so the test's
//!   fixtures stand for them: a fixed record of two preview outcomes for an outcome table, and a previewing entry
//!   that answers the acknowledgement's two fields for a named-word check. The checker decides them by capability
//!   separation (LG3.13) and the built-in column only; it has no rule for a recommendation read from state.
//! - **Provenance reader** (PS1.16, KN5.26; KN5's stage 3). [`workspace::Reader`] is the host's
//!   `c600_command::Provenance`. Its check reads a record's block with `c600_run::provenance::read`, so `workspace.set`
//!   refuses a block that is not a canonical FP.17 block with `PS1:capability.provenance_incompatible`. The check reads
//!   the block alone, so a record may hold a word that no preview admits, such as a solver tool's output (BND.84).
//!   Its admission is `c600_run::boundary::admit_preview` under [`tool::pins`], so a draft `{"record": name}` is
//!   refused with that function's code and detail: `LG3:capability.not_granted` under BND.84 and BND.85, and
//!   `PS1:capability.provenance_incompatible` for a pin the policy does not hold. The reader knows no session
//!   profile: it takes every session to be a solving session, the only profile LP3's host runs, so it applies BND.84
//!   and BND.85 at every record draft (KN5.26 asks for them in a solving session). It finds a part only among the
//!   blocks its host keeps ([`workspace::Reader::keep`], [`workspace::Reader::keep_run`]), and a part it does not
//!   find reads `unknown` and fails closed. No session table of blocks exists (M6's journal holds a record's block
//!   only as the record's provenance text), so a record write keeps nothing, and after a reopen the host keeps its
//!   blocks again. The reader drops no block it keeps, so its blocks grow with the runs of the host's session, a
//!   run's input and output blocks for each run of `c600-lp3-source`.
//! - **The host's table.** `c600-lp3-source` serves H-08 with the command layer's draft rows ([`workspace::table`])
//!   and installs a reader ([`workspace::install`]) that keeps each run's blocks, so a host can write and preview
//!   workspace records. The runner writes none, so its traces and receipts are as before.

#![forbid(unsafe_code)]

pub mod bridge;
pub mod case;
pub mod expand;
pub mod process;
pub mod supports;
pub mod tool;
pub mod views;
pub mod workspace;

/// The sources of the built-in tools, the bridge, the provenance reader, the trace relation's supports and the source
/// route's runner, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 12] = [
    ("c600-tools/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-tools/src/bin/c600-lp3-source.rs", include_bytes!("bin/c600-lp3-source.rs")),
    ("c600-tools/src/bridge.rs", include_bytes!("bridge.rs")),
    ("c600-tools/src/case.rs", include_bytes!("case.rs")),
    ("c600-tools/src/expand.rs", include_bytes!("expand.rs")),
    ("c600-tools/src/full_600_cell.6cs", include_bytes!("full_600_cell.6cs")),
    ("c600-tools/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-tools/src/process.rs", include_bytes!("process.rs")),
    ("c600-tools/src/supports.rs", include_bytes!("supports.rs")),
    ("c600-tools/src/tool.rs", include_bytes!("tool.rs")),
    ("c600-tools/src/views.rs", include_bytes!("views.rs")),
    ("c600-tools/src/workspace.rs", include_bytes!("workspace.rs")),
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
