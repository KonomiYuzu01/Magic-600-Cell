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
//! - [`kernel_rows`] holds the kernel rows that [`bridge`] serves to a program's snapshot read with an argument (KN5,
//!   CM.18): KN2.4's `positions` and `labels`, which LB2's exact completion in `c600-library` reads (CT-LB2-04),
//!   KN2.4's `effect`, which LB2's recipes and reports read (CT-LB2-01), and the `protection` row, which LB2's Net and
//!   Strict protection reads (CT-LB2-02), and the `setups` row of the setup-search exception, which LB2's guarded
//!   setups read (CT-BND-18).
//! - [`expand`] is the source route's recipe expansion: a case step's recipe to its literal word, through the kernel,
//!   outside the program and without a state read.
//! - [`case`] reads the frozen oracle cases O01 to O07.
//! - [`views`] builds the tool's views: the display contract's publication of a proposal and its preview, the content
//!   the headless view model installs for it, and the adapter's presentation report to the shell. Its
//!   [`views::ModelViews`] serves the command layer's view rows, `view.open` and `view.close`, with the view model.
//! - [`workspace`] is the host's provenance reader on the command layer (KN5's stage 3): it reads a workspace record's
//!   block before `workspace.set` writes the record (PS1.16), and admits or refuses a draft that names a record
//!   (KN5.26, BND.84, BND.85).
//! - [`modules`] is the host's side of a user module's runs (EV1.7, EV1.11; LP4-2): a program job of a user module's
//!   overlay row, or a view source that names its module digest, binds wholly to the module's active version in
//!   `c600_module::Modules`, and the binding runs in process or on the process host, staying on its version to its end.
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
//! - **Entry.** `full_600_cell.preview_word(w: Word)` is an exported command (LG3.6) that proposes `w` as given. The
//!   library entries of chapter 03 are language code in `c600-library`, not in this tool: LB1's literal words
//!   (`words.power`), LB2's exact completion (`completion.exact_completion`), LB2's recipes and reports (the module
//!   `recipes`), LB2's Net and Strict protection (the module `protection`) and LB2's guarded setups and buffers (the
//!   module `guarded_setups`); this crate's tests run these four through [`bridge`] over [`kernel_rows`] (CT-LB2-04,
//!   CT-LB2-01, CT-LB2-02, CT-BND-18).
//! - **Target.** The proposal's target text names the command-layer row the proposal reaches, `operation.preview`.
//! - **Result.** The entry returns `record { base, id }`, read with `receipt_base` and `receipt_id`: the two receipt
//!   fields a previewing program may read (BND.42). A receipt itself has no canonical output form.
//! - **Manifest and class.** The IR and the syntax carry no module manifest yet, so [`tool::Manifest`] is Rust data.
//!   The declared class is LB6.5's candidate for the word/recipe composer and full-effect inspector, `analysis` and
//!   `checking`; the rows are `operation.preview`. No `PrincipleRecord` and no review record exist (LB6.3, BM5), so
//!   the review of the class against the entry (CT-BND-15, CT-LB6-03) is not done.
//! - **Built-in admission.** `c600_check::admit` with `Profile::Solving` is the column of a solver's own, forked or
//!   imported tool, which prepares no preview until the joint setting, so it refuses this tool. [`tool::Tool::new`]
//!   admits the entry with `c600_check::admit_built_in`, the built-in column of chapter 07 section 8 that LP2 added to
//!   the checker (pull request 170) when this crate asked for it, and the process host's child admits it the same way
//!   (**Child admission** below); this crate keeps no admission rule of its own. Draft: BND.57's rows by class do not
//!   exist as data, so [`tool::class_rows`] lists the prepare-a-preview row of each of the manifest's rows,
//!   `operation.preview`, and nothing else. An entry that also reads state, publishes a view or writes the workspace
//!   therefore uses a row its class does not list and is refused, and a store of a word the solver did not name is
//!   refused at the write as well as its relay at the proposal. The built-in column's two exceptions are not built.
//! - **Provenance.** The blocks name a draft capability contract, pinned as `c600-tools-draft-1`, and the checker
//!   pin `c600-check-lp2-draft` over `c600-check`'s source. The word argument enters with a manual block: the solver
//!   named it. The producer is the tool's module digest and entry, author `built_in`, profile `solving`.
//! - **Limits** (KN7.7; LP2's pull request 348). A run gets the run limits of its job's budget
//!   ([`process::ToolCommand::limits`], [`tool::limits`]): the budget the solver's command declared, by default the
//!   shell's, every profile's ceiling ([`tool::LIMITS`]), and the layer admitted under its ceiling. A command that
//!   declares no budget, or one over the ceiling, is refused before a job exists, so the tool does not run. The
//!   bridge declares the run's budget on the runtime invocations of a row an entry serves
//!   ([`bridge::Bridge::with_budget`]). The run's charges are information only.
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
//!   length (LG2.17) with a result bound of [`bridge::RESULT_BOUND`] bytes; H-08 prices only reads (CM.18). The
//!   answer the shell sees is a receipt `{base, id, status, job}`; the layer's refusal or failure is its error code.
//!   The acknowledgement's `base` is `<session>@<epoch>/<head>`.
//! - **Kernel rows** (KN2.4, KN5; [`kernel_rows`]). The bridge serves four reads, each to a snapshot read with an
//!   argument, and a fifth when the host gives it the setup table, and no other: `positions` and `labels` at the
//!   layer's head, `effect`, a recipe's net map, which reads no state, and `protection`, a captured lock judged against
//!   a recipe at the head (the argument is `{lock: {kind, labels, slots}, mode, recipe, limit?}`). `positions` and
//!   `labels` each take a list of integers (a `List<Nat>` or a `Set<Nat>`, repeats allowed) and answer a list in the
//!   argument's order. `positions` answers one record per named piece, 0 to 177,119, through `CommandLayer::positions`;
//!   `labels` answers one label per named slot, 0 to 259,799, as `List<Nat>` or `List<Int>`, read from the head's
//!   snapshot because no layer or kernel function serves the row. A label is the home slot of the sticker the slot
//!   holds: a sticker's identity, never its colour. Both rows refuse in one order: a declared type the row does not
//!   answer, `LG2:type.mismatch`, and an argument that is not a list of integers, `KN5:input.argument_form`, cost the
//!   read's step only; then, charged, Jumble on is `KN5:input.retained_tool_needs_jumble_off` and a piece or slot out
//!   of range `KN2:input.named_object_invalid`. Tariffs: `positions` costs one pass over the 259,800 labels plus 20
//!   slots per named piece, and `labels` one unit per named slot; each result bound is the size of an answer of that
//!   many of the largest items. `effect` takes a recipe, a word or a list of word and star steps, and answers the
//!   fields its declared record names (the map, its digests, the counts and the support per orbit); it refuses in the
//!   same order, with the kernel's refusal of a recipe in place of a range error. The `setups` row, read only
//!   under the built-in column's setup-search exception, answers the certified setups for the target the solver names,
//!   and the bridge serves it only with the `SetupTable` the host gives it. Each row's argument, answer, codes and
//!   tariff are in [`kernel_rows`].
//! - **Source route.** `c600-lp3-source` keeps `c600-lp3-oracle`'s session names, shell prefixes, trace form and
//!   candidate block (`c600-command-lp3-loop` with the command layer's build), so the two routes' traces compare byte
//!   for byte; its summary adds the route `source`, this crate's [`build`] and the host. Each step's run is
//!   named `<case>-<index>`. The shell commits with `Ctrl+Shift+Enter`, the checked key map's chord for
//!   `operation.execute` (H-08's `key_0_4`), because the map binds `Ctrl+Enter` to `operation.preview`, which on
//!   this route the program invokes; undo and redo are `Ctrl+KeyZ` and `Ctrl+KeyY`, and a refused commit's preview
//!   is dismissed with `Ctrl+Shift+Backspace` (LP4-8).
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
//! - **View rows** (EV3.2, RPI.81 to RPI.83; LP2's pull request 271). [`views::ModelViews`] numbers each layer command
//!   ID it is called with as the view model's caller command, from 1 in the order it first sees it, and states the
//!   solver as the origin, since the layer calls it only inside a command the solver invoked. The host's declare
//!   function gives each source's dependencies, workload, output allowance and class, and refuses a source the host
//!   does not serve before the model is called (the tests' hosts with `KN5:input.argument_form`, naming `source`, as
//!   `ModelViews` refuses an overlay's module digest that is not 64 hex digits). An overlay's module digest reaches the
//!   model as its 32 bytes and its arguments as canonical JSON bytes. A model refusal keeps its code and names the view
//!   or the ceiling's field. A closed view's retirement token waits until the host takes it at its frame boundary.
//! - **Process host** (RTE.21, ONE.69, LQ9; KN5.15; BND.42). [`process::ProcessHost`] sends one job per run to
//!   `c600-runtime`'s child: the source bytes, the manifest's module and entry, the job's word, the run limits of the
//!   job's budget, the tool's class rows, the test tariffs, no grant and no source ranges, so the run's summary is the
//!   in-process host's.
//!   The boundary and the bridge stay in the caller's process as the `Host` of `c600_runtime::Runtime::run`: the child
//!   holds the program and its word, and no token, secret or command layer. One child serves every run of a host; after
//!   a fault the next run starts a fresh one (KN7.13). A run has no wall-clock deadline unless the caller sets one
//!   (KN7.10, DET.17).
//! - **Child admission** (ADR-L02, LG3.13, chapter 07 section 8). The job runs in the solving profile and names the
//!   tool's class rows (`c600_runtime::JobOptions::built_in`, LP2's draft), so the child admits the entry in the
//!   built-in column itself, with `c600_check::admit_built_in`, as [`tool::Tool::new`] does in the caller
//!   ([`process::job_under`]). The process host still sends only the source whose digest is the manifest's. Solving is
//!   the profile of the place where LP3's hosts start a run, a command in a solving session (BND.17); the tool editor
//!   and a research workspace, which would set the tool-authoring and research profiles, do not exist yet.
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
//! - **Oversize and chunking** (LB2.3, LB2.4, LB1.11). The bridge never cuts, splits or rewrites a word or a recipe.
//!   The command route's chunking is the 0.4 recipe's word steps, and the source route carries the solver's own
//!   recipe: for one run the host gives the bridge the input recipe, exactly as written, with its literal word
//!   ([`bridge::Steps`]), built only from a recipe the kernel expanded, whatever its steps. A proposal longer than one
//!   chunk of 100,000 letters ([`bridge::WORD_CHUNK`]) that is exactly that literal word, letter for letter, goes in
//!   the one invocation as `{"draft": {"recipe": <the solver's recipe>}}`, unchanged ([`bridge::proposal_args`]):
//!   plain word steps, star steps, and word steps with other fields, which are kept. Every other proposal is one word
//!   step, as before: a word of at most one chunk, whatever the recipe, so O01 to O07 send the invocations they sent
//!   before; and a word that is not the input literal, which the layer refuses whole when it is longer than one
//!   chunk, as it refuses the same draft on the command route. The tariff stays the literal word's length, so the
//!   total bound of 3,000,000 is unchanged, and one proposal stays one invocation, one preview job and one commit.
//!   The draft the bridge sends is the solver's recipe byte for byte, so the source route admits a recipe over one
//!   chunk exactly when the command route admits the same draft.
//! - **Net and Strict checks** (LB2.3, CT-LB2-02's Net and Strict halves). The tests take the Net check to be the
//!   layer's preview verdict at label level over a session's protected set, here the orbits one generator moves, and
//!   the commit's refusal when that verdict is not clear. The Strict check is the same after the shell sets the
//!   session's policy to Strict with `protection.policy.set` (LP2's pull request 198; the runtime may not invoke the
//!   row): the layer judges every executed literal prefix with the kernel's `strict_protection`, the preview's summary
//!   names the mode and the first refused prefix, and the commit is refused while a prefix moves a protected orbit. The
//!   captured lock and "an incomplete prefix check never returns safe" (LB2.3's coverage report and
//!   `LB2:protection.incomplete_coverage`) are built in `c600-library`'s `protection` module over the bridge's
//!   `protection` row, and run whole in `tests/ct_lb2_02.rs` (pull request 300); the bridge serves `positions`,
//!   `labels`, `effect` and `protection`, and `setups` with the setup table.
//! - **Inspector fixtures** (LB3.5, CT-LB3-07's checker half). No LB3 entry exists as library code, so the test's
//!   fixtures stand for them: a fixed record of two preview outcomes for an outcome table, and a previewing entry
//!   that answers the acknowledgement's two fields for a named-word check. The checker decides them by capability
//!   separation (LG3.13) and the built-in column only; it has no rule for a recommendation read from state.
//! - **Provenance reader** (PS1.16, KN5.26; KN5's stage 3). [`workspace::Reader`] is the host's
//!   `c600_command::Provenance`. Its check reads a record's block with `c600_run::provenance::read`, so `workspace.set`
//!   refuses a block that is not a canonical FP.17 block with `PS1:capability.provenance_incompatible`. The check reads
//!   the block alone, so a record may hold a word that no preview admits, such as a solver tool's output (BND.84). Its
//!   admission is `c600_run::boundary::admit_preview` under [`tool::pins`], so a draft `{"record": name}` is refused
//!   with that function's code and detail: `LG3:capability.not_granted` under BND.84 and BND.85, and
//!   `PS1:capability.provenance_incompatible` for a pin the policy does not hold. The reader needs no session profile:
//!   the layer asks it to admit a record's word only in a solving session, where KN5.26 asks for BND.84 and BND.85, and
//!   asks for its check at `workspace.set` in every profile (LP2's pull request 287). It finds a part among the blocks
//!   its host keeps ([`workspace::Reader::keep`], [`workspace::Reader::keep_run`]) and the part blocks the session
//!   keeps, and a part that neither holds reads `unknown` and fails closed. The session's part blocks are the layer's
//!   (CT-BND-27; LP2's pull request 272): at a record's write the reader lists the parts of the record's block that its
//!   host keeps, recursively, the layer keeps those it does not have with the action, and it gives them back at
//!   admission, so after a reopen a record written while the host kept its parts is admitted without the host keeping
//!   them again. The reader drops no block it keeps, so its blocks grow with the runs of the host's session, a run's
//!   input and output blocks for each run of `c600-lp3-source`.
//! - **The host's table.** `c600-lp3-source` serves H-08 with the command layer's draft rows ([`workspace::table`])
//!   and installs a reader ([`workspace::install`]) that keeps each run's blocks, so a host can write and preview
//!   workspace records. The runner writes none, so its traces and receipts are as before.

#![forbid(unsafe_code)]

pub mod bridge;
pub mod case;
pub mod expand;
pub mod kernel_rows;
pub mod modules;
pub mod process;
pub mod supports;
pub mod tool;
pub mod views;
pub mod workspace;

/// The sources of the built-in tools, the bridge and the kernel rows it serves, the provenance reader, the trace
/// relation's supports and the source route's runner, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 14] = [
    ("c600-tools/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-tools/src/bin/c600-lp3-source.rs", include_bytes!("bin/c600-lp3-source.rs")),
    ("c600-tools/src/bridge.rs", include_bytes!("bridge.rs")),
    ("c600-tools/src/case.rs", include_bytes!("case.rs")),
    ("c600-tools/src/expand.rs", include_bytes!("expand.rs")),
    ("c600-tools/src/full_600_cell.6cs", include_bytes!("full_600_cell.6cs")),
    ("c600-tools/src/kernel_rows.rs", include_bytes!("kernel_rows.rs")),
    ("c600-tools/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-tools/src/modules.rs", include_bytes!("modules.rs")),
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
