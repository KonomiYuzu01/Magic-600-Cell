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
//!   for byte; its summary adds the route `source`, this crate's [`build`] and the host. Each step's run is
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
//!   kernel's. `c600-run`, `c600-check`, `c600-syntax`, `c600-ir` and `c600-runtime` have no build string yet, so it
//!   does not cover them.
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
//! - **Words with no transfer encoding** (`c600-runtime`'s draft 7, `PS1:input.noncanonical_value`). A word with a
//!   letter outside 1 to 1200 in magnitude cannot cross to the child: the job is refused at the start stage and
//!   nothing is proposed. In process the same word runs and the command layer refuses its proposal. The runner takes
//!   the start-stage refusal as an `invalid-input` step only for a recipe the kernel already refused, and keeps the
//!   kernel's refusal in the step record. In O07 that is steps 0, 1, 2 and 8: the traces are the same on both hosts,
//!   and those four steps have no receipt on the process host. It is the one difference between the hosts' records
//!   beyond the host's name.
//! - **Host in the records** (plan section 7). `--host in-process|process`, in-process by default; `--runtime` names
//!   the runtime's binary, by default `c600-runtime` beside the runner (packaging decides later). The summary has
//!   `host` and `runtime_spawns`, each case of the summary `start_refused`, each case of the receipts file `host`,
//!   and each step record `host` (null when no word was given to a host) and `start_refusal`.
//! - **Oversize and chunking** (LB2.3, LB2.4, LB1.11). The bridge never splits a word: one proposal is one word
//!   step, whatever its length, so the layer refuses an oversized word whole, as it refuses the same draft on the
//!   command route. The command route's chunking is the 0.4 recipe's word steps. The tool's entry takes one flat
//!   word, so a valid chunked recipe of more than 100,000 letters is refused on the source route; carrying one needs
//!   a list of words in the IR or KN2.10's streaming.
//! - **Net check** (LB2.3, CT-LB2-02's Net half). The tests take the Net check to be the layer's preview verdict at
//!   label level over a session's protected set, here the orbits one generator moves, and the commit's refusal
//!   when that verdict is not clear. The kernel has no Strict mode, so the Strict half is not built.
//! - **Inspector fixtures** (LB3.5, CT-LB3-07's checker half). No LB3 entry exists in the IR, so the test's
//!   fixtures stand for them: a fixed record of two preview outcomes for an outcome table, and a previewing entry
//!   that answers the acknowledgement's two fields for a named-word check. The checker decides them by capability
//!   separation (LG3.13) and the built-in column only; it has no rule for a recommendation read from state.

#![forbid(unsafe_code)]

pub mod bridge;
pub mod case;
pub mod expand;
pub mod process;
pub mod tool;

/// The sources of the built-in tools, the bridge and the source route's runner, hashed into [`build`].
pub const SOURCES: [(&str, &[u8]); 9] = [
    ("c600-tools/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-tools/src/bin/c600-lp3-source.rs", include_bytes!("bin/c600-lp3-source.rs")),
    ("c600-tools/src/bridge.rs", include_bytes!("bridge.rs")),
    ("c600-tools/src/case.rs", include_bytes!("case.rs")),
    ("c600-tools/src/expand.rs", include_bytes!("expand.rs")),
    ("c600-tools/src/full_600_cell.6cs", include_bytes!("full_600_cell.6cs")),
    ("c600-tools/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-tools/src/process.rs", include_bytes!("process.rs")),
    ("c600-tools/src/tool.rs", include_bytes!("tool.rs")),
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
