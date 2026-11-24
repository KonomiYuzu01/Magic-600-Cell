# 600 Cell Script in Rust

The Rust workspace of 600 Cell Script, built under LP2 ([plan](../docs/progress/1.0/language-lp2-plan.md)) and LP3
([plan](../docs/progress/1.0/language-lp3-plan.md)). The
engine, the session store, the command layer and the interpreter's host are written in Rust (LQ8), and the language
runs on its own interpreter over the typed IR (route RT1, LQ9). The frozen architecture is in
[`docs/architecture/1.0/language/`](../docs/architecture/1.0/language/); nothing here changes it.

## Crates

| Crate | What it holds |
| --- | --- |
| `c600-ir` | The typed IR draft `c600-ir-draft-1`: data types, and short constructors for hand-built test programs |
| `c600-check` | The checker over the IR: base types (LG2.1), exactness (LG2.19 to LG2.22), effects, capabilities, views and capability separation (LG3), report order and contract refusal (LG4.1, LG4.3) |
| `c600-syntax` | The parser of `.6cs` source files: source byte checks, parsing with recovery, the canonical layout, and lowering into the IR draft with a source map (LG1, LG4.12) |
| `c600-digest` | SHA-256, raw and in hex, for the other crates |
| `c600-run` | The interpreter over the IR draft (route RT1) with the `lp1-exec-draft-1` reference meter and generator (M2), and the run-time boundary under capability separation with FP.17's provenance blocks (M3) |
| `c600-kernel` | The 600-cell-Full kernel read straight from `assets/`, its chapter 01 API, and the oracle adapter `c600-oracle-adapter` (M4) |
| `c600-jumble` | The jumbling half of the kernel API (M5): exact poses over Q(√5), K⁺, piece regions, twists and menus, states with their pose tables, the observations, and the adapter `c600-jumble-adapter` that the differential harness drives |
| `c600-session` | The session store below the language: the journal, revisions, snapshots, the commit path, undo, branches and program jobs (M6) |
| `c600-evidence` | Chapter 06's evidence library as pure code: statements, keys and scopes, the AND/OR graph, grades and editions, the two standing passes, the word function with EVD.32's rows and EVD.40's checks, `CheckReport` admission, EVD.52's import rules and EVD.64's writer check |
| `c600-command` | The command layer over H-08's stored table (KN5, chapter 08 section 7), the shell's tokens and key routes, and the loop runner `c600-lp3-oracle` (LP3, C1) |
| `c600-view` | The headless view model, view lane and view budget: Global, Local and further views, revision binding and statuses, visibility, and the views' limits and allowances (LP3, C2) |
| `c600-display` | The headless display contract: display plans and publications, the host validator, the publication state machine over a mock backend, picks and frame stamps, RM.37's jumbling crosswalk as fixtures (LP3, C2), and the jumbling producers from M5's kernel (LP3, C8) |
| `c600-module` | Module closures, lockfiles and their approval binding: the draft manifest and lock forms, resolution by content digest, closure verification and the closure digest, an approval bound to that digest, and load and run through the run-time boundary (LP3, C4); a host's module set with version separation and activation (LP4-2) |
| `c600-runtime` | The runtime in a process of its own (route RT1): the child binary `c600-runtime`, the parent library that starts it and answers its host calls, the wire protocol draft `c600-rt1-wire-1` and the in-process reference path (LP3, C3) |
| `c600-durable` | LP3 C7's durable conformance cases on KN5's command layer over M6's journal and on M6's session store, test-only: CT-RTE-08's persisted word, CT-EV3-07's durable action order and view part, CT-LB2-03's work-sheet part, and the durable leg of LP2's saved sets (LP3, C7) |
| `c600-tools` | The built-in 600-cell-Full tool as `.6cs` source, its admission under the built-in column, the host bridge from the runtime onto the command layer, in process or on the runtime's own process, each run under its job's budget, with KN2.4's `positions`, `labels`, `effect`, `protection` and `replay` rows, the draft `setups` and `orbits` rows and the representations tool's draft `structure.compare`, `filter.preview` and `structure.layer` rows (C6) as snapshot reads, the recipe expansion, the source route `c600-lp3-source`, the tool's views, the host's view rows on the view model and the host's provenance reader (LP3, C5), and the loaded model's trace relation (LP2 draft, LB1.10) |
| `c600-repr` | The representations tool, headless half: layouts and atlases with their source maps, the layout filter, linked selection, chart surveys and user-chart admission over the display contract, the tool's `.6cs` source and `view_aid` manifest, its class review, and its six rows: what they need from the command layer, and all six served there by a test host through the command and source routes, whose two snapshot reads `c600-tools`' bridge answers from LP2's structure rows, and the applied filter's display (LP3, C6) |
| `c600-jumble-tool` | The built-in jumbling tool, headless half: its `.6cs` source with two entries that preview a word the solver names, its `checking` manifest and built-in admission, literal jumbling words and their constructors, levels, protection verdicts bound to their level, a slot's current occupant on and off the lattice through M8's slot action of K⁺, the relation `jumble-locality/1` for LB1.10's trace keys, LB3's ν and drafted correction-set entry on F2 and F5 (`jumble_nu`), the repair list at a named cap under the built-in column's repair-list exception (`jumble_repair`), the three end tests read as their own verdict types (`jumble_end_tests`), the jumbling rows' host bridge (`rows`: KN2.4's jumbling rows and `seam_facts` as snapshot reads, also at the command layer's head with Jumble on), and the jumbling halves of LB1's, LB3's, LB4's and EV3's tests on M8's levels and repair list, with oracle equality through Jumble on in the command layer (LP3, C8) |
| `c600-input` | The headless input router: synthetic key events to command-layer invocations in chapter 04's routing order, bindings resolved to absolute arguments at the press, the held-key allowlist and the key-map checks (LP3, C5) |
| `c600-library` | The base library's first entries as 600 Cell Script, with their catalogue rows: LB1's literal words (`words.power` in `words.6cs`); LB1.8's F2 and F5 ranks and solutions (the module `linalg` in `linalg.6cs`), exact Q(√5) signs (`quadratic`) and quaternion factors and tree outputs (`trees`), CT-LB1-01's parts, and its word equality keys (`word_keys`), CT-LB1-02's 600-cell-Full half on the model's relation in `c600-tools`' tests; LB2's exact completion (`completion.exact_completion` in `completion.6cs`), recipes and reports (`recipes`), Net and Strict protection (`protection`), guarded setups and buffers (`guarded_setups` in `setups.6cs`), named macro analysis and comparison (`macros`), seeds with their collateral stage order (`seeds`) and the orbit catalogue, progress per orbit and residuals (`orbits`), whose entries that read state run whole through the tool's host bridge in `c600-tools`' tests (CT-LB2-04, CT-LB2-01, CT-LB2-02 and CT-BND-18 among them); and CT-LB1-03's 600-cell-Full half and CT-LB2-02's Strict half on the kernel (LP3, C5) |
| `c600-platform` | The platform layer (RPI.52), the one member that calls an operating system's own API: the session lock file's open, which on Windows denies delete sharing so that a held `lock` keeps its name (E1, the owner's reading of 18 November 2026), and the start of a console program without a console window of its own (`no_console_window`, for `c600-runtime`'s child). std only |
| `c600-build` | The language evidence builder and its build cache: the scoped output grant, sealed builds on the caller's thread, and the fail-closed cache (DET.44 to DET.46; M2) |
| `c600-render` | The display host's bridge to the S-A2 renderer core: `c600-display`'s `Backend` over the core's ABI 5 through a safe call interface, with a fake core for headless tests, and user stages as HLSL under `m600-stage-hlsl/1` with their binary32 CPU reference; the table of the core's C functions is in `renderer/bridge`, outside this workspace (LP3, C9) |
| `c600-lx4` | LX-4's module set (S, M, L and the 50-module W workspace) and seeded edit script, generated from the three LP3 tools and checked by the front end before anything is written; measurement tooling, not product (LP3, C9) |
| `c600-lx2` | LX-2's metered and forced-kill trials, alone and with a session open, against a runtime child binary the caller names (the product build's on the owner's machine), and the allocator-ceiling, native recursion and native loop trials on its injection device `c600-lx2-child`; measurement tooling, never shipped (LP3, C9) |
| `c600-editor` | EV1's editor services as a library: digest-pinned documents and source ranges with host-unit conversion, parse with recovery and trivia, completion, diagnostics, hover, definition and rename, each diagnostic and refusal as typed data, for the C ABI host library of LP4-3 (LP4-1) |
| `c600-debug` | EV2's read-only debugger, replay and cost profile: stepping RT1's evaluator at the run's own LG4/DET.20 positions with suspended kernel calls, value inspection, word references into compressed words, replay with preflight and mismatch reporting, journal travel over a host's query rows, a pinned run's freshness and checker-trace stepping (LP4-5, LP4-6) |
| `c600-log` | The 1.0 log file: what a log holds, its record `c600-log-draft` (`.c600log`), the compiler with its gates, a recorder, and the check, import and export of 0.4 proof logs and MPUlt v1 logs of 600-cell-Full (LP4-12, LP4-13); no path to `c600-run` |
| `c600-docs` | EV5's reference docs and principle records: reference entries generated from checked metadata, an entry's decomposition from the checked IR's call graph, `PrincipleRecord` data read and admitted against it (typed nones, resolving digests, derivation-backed claims, stale hand parts), and explanations whose words come from chapter 06's word function, with lazy provenance links and forks that inherit no evidence (LP4-10) |

| `c600-checkers` | KN6's independent checkers as kernel code: one per certificate kind, the research-archive kind as a family of sub-checkers with their own identities and mutation gates (LP5, R2 to R6; scaffold) |
| `c600-archive` | PS3's research archive import and exchange: the versioned import table, records with no format key, manifest and member checks, the import report, the typed claim bindings and the check-bundle export; it executes nothing (LP5, R0 and R1; scaffold) |
| `c600-research` | The research host: research overlay rows and host reads, the KN6 check job adapter, reproducible research jobs on engine-owned branches, the obligation export row and the research output panel's rows (LP5, R8 to R10; scaffold) |

The checker depends on the IR crate only, and the syntax crate on the IR and digest crates; the syntax crate's tests
also run the checker. The interpreter depends on the IR and digest crates and the big-integer crates, not on the
checker; its tests run checked source programs, so the checker and the syntax crate are its dev-dependencies. The
kernel depends on the digest crate only. The session crate depends on the kernel, the interpreter, the IR, digest and
evidence crates and on the command layer; its job tests run checked source programs too, and its jumbling tests read
the twist menu through the jumbling crate, a dev-dependency. The evidence library depends on the digest crate only.
The builder depends on the interpreter, the IR and digest crates: it is the host envelope that reads captured inputs
and writes the build cache and the published files, so the interpreter itself keeps no file-system call (CT-KN4-01).
The command layer depends on the kernel and digest crates only, so it has no path to the interpreter (ONE.48). The
view model depends on the digest crate only, and the display contract on the digest, kernel and jumbling crates.
The module crate depends on the syntax, IR, checker, interpreter and digest crates, and has no path to the command
layer.
The runtime depends on the syntax, checker, interpreter, IR, digest and module crates and the big-integer crates, and
has no path to the command layer; the module crate gives it the binding a bound job is built from (LP4-2).
The tools crate depends on the syntax, checker, interpreter, IR, digest, kernel and module crates, on the command
layer, and on the runtime, display and view crates: its host bridge connects the interpreter, in process or in the
runtime's own process (C3), to the command layer as the runtime's host, its views build on the display contract and the
view model, and its user-module runs bind through the module crate's set (LP4-2). It also depends on the debugger
crate, whose journal rows it serves over the command layer and whose stepped runs of a binding it launches (LP4-5,
LP4-6). The library crate is its
dev-dependency, so that its tests run the library's exact-completion entry through the host bridge, and so is the input
router, so that its tests invoke a user module's row from a key.
The input router depends on the command layer and the kernel crate only, so a key reaches a kernel row with no path to
the interpreter (ONE.48).
The jumbling crate depends on the kernel and digest crates and the big-integer crates.
The representations crate depends on the checker, whose built-in column admits its entries, the IR and digest crates,
the display contract, the tools crate and the command layer, whose stored table its class review reads; it draws
nothing and owns no device.
The jumbling tool crate depends on the digest crate, the jumbling crate and the tools crate, whose `Tool` admits its
entries under the checker's built-in column; its tests also run the checker, syntax, IR, interpreter and kernel crates.
The durable crate holds tests only; its dev-dependencies are the session store, the module crate, the checker, the
syntax crate, the interpreter, the kernel and the digest crate.
The library crate depends on the IR, syntax and digest crates only; its tests also run the checker, the interpreter
and the kernel.
The editor services crate depends on the syntax, checker, IR and digest crates only, so it has no path to the
interpreter or the command layer and runs no user program (EV1.9).
The debugger crate depends on the interpreter, syntax, IR and digest crates only, so it has no path to the command
layer's writes, the session store, the runtime, the module host or any producer code (EV2.1, EV2.6, EV2.8); the
checker is its dev-dependency.
LP5's KN6 checkers depend on the kernel, jumbling and digest crates only, so no checker reaches the interpreter, the
library, an archive reader or a producer (KN6.2). The archive crate depends on the digest crate, the evidence library
and the kernel crate, whose container readers (DEFLATE, gzip, JSON) it reuses; it calls no engine. The research host
depends on the tools crate, the command layer, the session store, the library, the evidence library, the checkers and
the archive crate; the command layer gains no edge (ONE.48). Any other edge follows PHS.52 ([LP5
plan](../docs/progress/1.0/language-lp5-plan.md), section 4).

## Running the tests

Run `cargo test --locked --offline` in this folder. CI runs it through `tests/test_language_rust.py`, which
`tools/ci/run_headless.py` lists; a missing Rust toolchain fails that check. Build output goes to `target/`, which Git
ignores. Each crate keeps its tests in its `tests/` folder, and a test that covers part of a conformance test names
the ID and the part (PHS.62). The exactness tests read the frozen
[`vectors.json`](../docs/architecture/1.0/language/contract-vectors/vectors.json) in place.

## Dependencies

Only the Rust standard library and the crates below. An external crate needs the owner's approval first (RTE.70), and
each approved one is listed here with its licence.

Approved on 16 November 2026 for LP2's big integers
([owner decision](../docs/wiki/decisions/owner-decisions-2026-11-16-big-integers.md)); M2 added them, pinned exactly
with the archive checksums of LX-5's crate run. The archives are vendored in `vendor/` with source replacement in
`.cargo/config.toml`, so the tests need no network:

| Crate | Version | Licence, read from the verified archive |
| --- | --- | --- |
| `num-bigint` | 0.5.1 | MIT OR Apache-2.0 |
| `num-integer` | 0.1.47 | MIT OR Apache-2.0 |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 |
| `autocfg`, a build dependency of `num-traits` | 1.5.1 | Apache-2.0 OR MIT |

## Status of M1: the IR and the checker

- The IR is a draft and changes freely until the IR freeze (PHS.49). The checker's tests build IR by hand; the syntax
  crate's tests write the nine frozen exactness cases in source as well (M7).
- The IR and the checker advertise the exactness and execution contract `lp1-exec-draft-1` and refuse a program pinned
  to another contract or IR version with `LG4:input.ir_version_unsupported`. Form 2 (`lp1-exec-region-draft-1`) is not
  built, so CT-LG2-05's form-2 half is not applicable, which is no pass (PHS.63).
- Capability separation: the checker records each entry's separation form (reads state, prepares previews) while the
  flow rule is off (BND.41) and never emits `LG3:capability.flow_refused` before LQ2 sets that rule (CT-BND-24,
  CT-ERR-15). Under chapter 07's interim grant table an own, forked or imported tool prepares no preview in a solving
  session until the joint setting (BND.56, LG3.16), so the checker refuses one with `LG3:capability.not_granted`,
  also when an unresolved call or an open effect variable may prepare one.
- What an own tool may read in a solving session stays open (BND.59): the checker reports the open cell and neither
  grants nor refuses it.
- The built-in tool column of chapter 07's grant table (draft, LP2; LP3 asked for it): `admit_built_in` admits one entry
  of a built-in tool in the solving profile against the rows its reviewed class lists (`Row`, BND.57); the IR has no
  manifest, so the caller supplies them, and each summary lists the rows it may use (`Summary::rows`). It refuses with
  `LG3:capability.not_granted` an entry that is not exported, an ungranted capability (`LG3:capability.ambient_read`
  where `Summary::ungranted` says so; PHS.52 below), an entry that both reads state and prepares previews, a row the
  class does not list or the checker cannot name (an unresolved call, an open effect variable), and a proposal or a
  word-carrying workspace write whose word the solver did not name to the entry, unless an enabled declassifier covers
  the write; a write at a type variable, of a compressed word or a word equality key, or
  of a record, list, set, map or callable type that holds a word or a type variable, counts as one that may carry a
  word. The rule is conservative: it also refuses a named word passed to a helper that proposes or writes it, a write
  of a record, collection or closure built from named words, and a proposal to a `Target` parameter (a row the
  checker cannot name). The column's two exceptions are below. Not built: the derived-state row outside them (the IR
  draft has no `derive`), rule 6's check of the facts a view shows and the class review (CT-BND-15). `c600-runtime`'s
  job options carry the class rows (`JobOptions::built_in`), so its child admits a built-in entry itself.
- An entry that is no command and changes the workspace or prepares a preview is `LG3:effect.undeclared` at `entry`
  (LG3.5, LG3.6), on the built-in column and, for a workspace change, in `admit`'s solving profile. This confirms and
  closes LP3's finding of an exported plain `fn` that proposes: `check` admits it, since a command may call such a
  helper. In tool authoring and research `propose` goes to a capture (BND.60), and the frozen text does not say whether
  such a run needs a command entry, so `admit` still admits one there.
- Owner decisions of 16 November 2026 under PHS.52 (`../docs/wiki/decisions/owner-decisions-2026-11-16-phs52.md`). A
  capability the run does not hold, or that 1.0 never grants, is `LG3:capability.ambient_read` when it is the clock,
  the network, or the file capability of an entry that may read a file (a file read, or a call of a callable whose
  effect bound holds the file capability), and `LG3:capability.not_granted` otherwise; a manifest's network request is
  an ambient read. One mapping, `Summary::ungranted`, serves `admit`, `admit_built_in`, the manifest finding and
  `c600-module`'s approval of a built-in tool. Dynamic code, a call that resolves to no checked function, is
  `LG3:capability.not_granted` at the call (DET.36, RTE.45); a call of a function value whose callable the checker
  cannot see keeps `LG3:effect.undeclared`. The value of `use_goal` and `use_evidence` must be portable exact data: a
  type that holds a function, a receipt, a word equality key or a compressed word at any depth, or a type variable, is
  `LG2:type.mismatch` at the sink, and an approximate value keeps its exactness codes. Chapter 13 changed once, in the
  amendment LP3 made at its command-table freeze (PHS.53, chapter 13 section 9).
- Base types (LG2.1). A value whose base type does not fit its use is the diagnostic `LG2:type.mismatch` at the path
  of the offending expression, with the expected and supplied types in source notation (`Diagnostic::mismatch`); the
  exactness fields stay empty. This covers arguments, conditions, branches and arms, patterns, operands, field reads,
  results, workspace writes and reads, previews, receipt reads, budgets and calls of a value that is no function. A
  result is reported at the expression it returns, the rest of the innermost `let`. A literal pattern has no path of
  its own, so a pattern that does not fit the scrutinee is reported at the match; it compares as an exact equality
  does, so an `Int` scrutinee takes a `Nat` pattern. The IR draft has no identity-tower levels, so the types carry
  none and `LG2:type.level_mismatch` (LG2.3) does not arise yet.
- One fault, one finding. Where a value's base type does not fit and an exactness rule would also apply to it, the base
  type comes first: the finding is `LG2:type.mismatch`, and the value's exactness is not checked, because the exactness
  rules assume a value of the right base type. LG2.22's precedence orders the exactness codes only, so this is a draft
  choice (16 November 2026) that freezes with the IR (PHS.49). The value continues as one of unknown type (`_`), which
  fits every use and gets no check of its own, so no finding follows from it; a fault of the place, such as an exact
  budget set under approximate control, is still reported. A conversion has its own result type: an `approx53` of a
  `Bool` is a mismatch, and its `Approx<P53>` result at an exact operand is `LG2:type.exact_required` as well.
- Still an `IrError` that refuses the whole program, because chapters 02 and 13 give it no code: an unbound name, a
  function declared twice, an unknown or unexported function, a named function called with a wrong number of
  arguments, and effect inference that stops before its fixed point. Lowering refuses all but the last at their
  source text, so from source only the last reaches the checker. A function value called with another number of
  arguments is `LG2:type.mismatch` at the called value: the count is part of its type, which lowering does not know.
- Qualifiers through calls (LG2.19): a callable enters a slot only if its signature keeps every qualifier (its result
  no more qualified than the slot's, each parameter taking what the slot passes), and a function's own result or
  stored field is checked against its declared bound even when that bound has effect variables. The checker does not
  unify type variables: a result of a bare type variable keeps the qualifier, callable and fields of every argument
  bound to that variable, also through a callable's result, and binding one variable to values of incompatible types
  is `LG2:type.mismatch` at the later argument, with the earlier binding's type expected; any other generic result is
  display-qualified when a generic argument is, and a display value for a generic parameter makes the call a pure
  display body. A call instantiates the effect variables of its result type, and a closure's own effect variables are
  kept apart from captured ones. Records keep their field values, so a callable keeps its effects and exact sinks
  through a record.
- Approximate control through calls (LG2.20, LG2.22): a helper whose `Approx<p>` result is selected under approximate
  control passes that control to its callers, so an exact use of the result is `LG2:type.exactness_violation`, not
  `LG2:type.exact_required`; a display-qualified result shows its qualifier instead, so a later exact use erases it. A
  call inside a `display_if` branch to a function that may reach the caller again, directly or through a function
  reference, is refused: approximate data cannot bound recursion. A workspace or snapshot read keeps the display
  qualifier of every field the program stores under that name, and a read at a type variable keeps the stored
  qualifier itself, also in a record field (LG2.19; lead L1 of the M7 attack, 16 November 2026).
- An approximate literal as a match pattern is `LG2:type.exact_required` at the match: it is never read as exact
  (LG1.7), and a pattern compares as an exact equality does (lead L2 of the M7 attack).
- The three known limits of M1, leads from the Fable attack (owner decision, 15 November 2026), each confirmed by an
  attack test (`c600-check/tests/limits.rs`). Closed: an effect variable of a view's callable parameter that a call
  site instantiates with a workspace or preview effect is `LG3:effect.undeclared` at the call (LG3.5, LG3.6), also
  through a function value or a record field; recursion through a callable passed to a parameter whose bound has no
  effect variable is traced (LG2.20); and a function value or closure keeps its approximate-control bit, so an exact
  use of its result is `LG2:type.exactness_violation` (LG2.22). Open: a callable known only by its declared type
  carries no control bit, since the IR draft's function type has no place for one, so an exact use of its
  approximate result is `LG2:type.exact_required`; the refusal holds and only the code differs until the IR freeze
  (PHS.49). A further lead, not yet fixed: recursion steered inside the callee, where a helper calls its callable
  parameter in a `display_if` branch and is passed a function that reaches the helper again, is admitted.
- Not in M1, because they need the runtime (M2, M3): the fixed acknowledgement of BND.43, the saved-word half of
  CT-LG3-07, the undo half of CT-LG3-03 and research output that never becomes a solving preview (BND.61). M3 builds
  the acknowledgement, the saved-word half and BND.61's refusal at run time (below); the undo half of CT-LG3-03 runs in
  `c600-command` since LP2's reading 18 (the job output, below).
- Lists and word operations (LP2 drafts until the IR freeze, PHS.49). `List<T>` is a finite sequence in written
  order, built by the IR's collection form; `length`, `index`, `push` and `concat` work on it. A list with a display
  element is display-qualified as a whole, so its length, an element or an equality cannot extract an exact result
  (LG2.19), and an index takes exact inputs only (LG2.20): CT-LG2-05's aggregate part for lists. A list holds no
  callable in the draft. `inverse`, `conjugate`, `commutator` and `compose` (with `concat` and `length`) take the
  `Word` domain and give a `Word` (LG2.15, LG2.16; CT-LG2-12's language half, part). `word_key` gives a
  `WordEqualityKey` (LG2.13), which no proposal, word operation, word slot or result takes:
  `LG2:type.word_key_not_executable`, not a mismatch (CT-LG2-11, part). The key is not indexed by puzzle and identity
  yet, since the IR draft has one puzzle.
- Sets and maps (LP2 drafts until the IR freeze). `Set<T>` and `Map<K, V>` are built by the collection form; a map's
  items alternate keys and values. A set element and a map key have an exact type with a canonical key order (no
  display qualifier, approximation, function, receipt, `Unit` or type parameter, at any depth), or the IR is
  malformed, and every element, key and probe is an exact input (LG2.20). `contains`, `insert` (a set's element or a
  map's key and value), `lookup` and `length` work on them; a map with a display value is display as a whole
  (LG2.19). `elements`, `keys` and `entries` give a list in the order their primitive names, ascending or descending
  (LG4.3). CT-LG2-05's aggregate part for sets and maps.
- Compressed words (LP2 drafts until the IR freeze). `CompressedWord` is a type of its own, never a `Word`:
  `compress` gives one from a `Word`, `concat` joins two without expansion, `length` gives the exact `Nat` length
  without expansion (LG2.16), and `expand(c, start, count)` gives the letters from an exact position as a `Word`
  (LG4.6); the position and the count are exact natural numbers (LG2.20). A compressed word has no exact equality,
  alone or held in a record or a collection, and is no set element or map key, so no checked program compares two.
  CT-LG2-14's language half, static part (`c600-syntax/tests/collections.rs`).
- F2 and F5 (KN3.2, ADR-L03.2; LP2 drafts until the IR freeze). `F2` and `F5` are exact number kinds without a
  literal (LG1.6): `ToField` reduces an exact `Nat` or `Int` into one, `FieldAdd`, `FieldSub`, `FieldNeg`, `FieldMul`
  and `FieldInverse` stay in one field, and `Residue` gives an element's residue as a `Nat`. Nothing widens into or
  out of a field: mixing the two fields, or a field and another number kind, is `LG2:type.mismatch` at the operand,
  and an approximate operand of `ToField` is `LG2:type.exact_required`. `ExactAdd`, `ExactMul` and `ExactLess`
  refuse elements, which stay apart from the widening, and a finite field has no order; `ExactEq` compares two of
  one field. An element may be a set element or a map key (LP2's choice): its key order is the field, F2 first, then
  the residue. It holds no word, so the built-in column's rules treat it as plain data. It has no canonical form (FP.6
  wraps only int, rat and q5), so a goal, an evidence sink or a snapshot read's argument refuses it, also inside a
  record or a collection, and takes its residue.
  The tests are in `c600-check/tests/fields.rs`. CT-LB1-01's F2 and F5 ranks and solutions are LP3's library code
  (LB1.8): `c600-library`'s `linalg`, since pull request 274.
- Snapshot reads with an argument (LP2 draft until the IR freeze; LP3's plan, section 2). `ReadSnapshotWith` reads
  the engine's row `query` for an argument, such as a word's protection, for LB2's star and recipe construction and
  Net and Strict protection. The argument is portable exact data, as a goal or evidence is (owner decision of 16
  November 2026 under PHS.52): a function, closure, receipt, word equality key or compressed word is
  `LG2:type.mismatch`, and a display-qualified or approximate value, also inside a record or a collection, is
  `LG2:type.exactness_violation` (LG2.19, LG2.20). The read has the literal read's effect, `Read(query)` row and
  state-read fact, so the built-in column and capability separation count it, and its declared type stands: the
  engine answers it, never a workspace record. The literal `ReadSnapshot` keeps its shape
  (`c600-run/tests/snapshot_arg.rs`).
- Menu words (LP2 draft until the IR freeze; LB1.8, LP3's plan, section 2). `MenuWord` is the type of a word of the
  run's pinned jumbling menu, and `Lit::MenuWord` a literal of letters `[cap, item]`, each index below
  `MENU_INDEX_LIMIT` (600). `concat`, `inverse`, `conjugate`, `commutator` and `compose` take menu words as they
  take Full words and give a menu word, `length` counts its letters, and `word_key` gives a menu word's
  `WordEqualityKey` under the run's pinned menu matrices (LG2.18; `c600_run::menu_key`, LP2's pull request 338); the
  first operand of a known type sets an operation's alphabet. A Full word and a menu word never meet: an operation,
  collection, call, return, equality or match that mixes them, also inside a list, record, map or callable type, and
  a menu word given to `compress`, is `LB1:input.incompatible_alphabet` with the expected and the supplied type, not a
  mismatch; types that differ in more than the alphabet stay a mismatch. A menu word is proposed as a word is, with
  the `preview` effect, and the built-in column takes a `MenuWord` parameter as a word the solver named
  (`c600-check/tests/built_in.rs`, `c600-syntax/tests/menu_words.rs`).
- End tests' verdicts (LP2 draft until the IR freeze; LB3.2, LG2.9; LP3's plan, section 2). `Ty::EndTest` is the
  type of the verdict of one of LB3.2's three end tests, `RetainedCheckpointVerdict`, `LabelCheckpointVerdict` or
  `StickerTestVerdict` in source, which a program reads from the test's engine row `retained_checkpoint`,
  `label_checkpoint` or `sticker_test` (KN2.4), with or without an argument. The end tests' bare names stay chapter
  02's goal names (LG2.10). A verdict's outcome is `holds`, `fails` or `unestablished`, and a program uses it
  through a `match` on those tags only: a verdict used as a boolean (a condition, an operand of `and`, `or` or
  `not`, a `Bool` argument, result, element or field), a boolean where a verdict is expected, an exact equality
  that holds one, a boolean pattern, a read of an end test's row declared `Bool`, and a `_` arm that takes
  `unestablished` together with another outcome are `LG2:type.verdict_required`; one test's verdict where another's
  is expected, a read of another row declared a verdict, a status tag given for a verdict and another literal pattern
  are `LG2:type.mismatch`. No literal makes a verdict, and a verdict is no set element or map key
  (`c600-syntax/tests/end_tests.rs`: CT-LG2-09's part for end tests, CT-LB3-01's checker half). The checker does
  not unify type variables, so a generic function may compare a verdict it cannot see; M2 refuses that at run time.
- Shown facts (LP2 draft; LB3.5, DT-J1 rule 6). In an entry that reads state-derived data, `admit_built_in`
  refuses with `LG3:capability.not_granted` each expression the body may return, when the result type is
  word-carrying, and each view of a word-carrying value, unless it is a `Word` or `MenuWord` parameter of the entry
  as given; a function the entry reaches that publishes a word is refused at the entry. In every entry it refuses a
  snapshot read whose argument is not a parameter of the entry as given, a literal, or a record or a collection of
  those (no prefetch for an unnamed cap), and a function the entry reaches that reads for anything but a literal. A
  selection that is no word, such as a cap the method chose, is not told from a per-cap fact
  (`c600-check/tests/built_in.rs`: CT-LB3-07's checker half for recommendations and reads).
- The built-in column's two exceptions (LP2 draft; BND.1, BND.2, BND.69; chapter 07 section 8). A class grants one by
  listing `Row::RepairList(query)` (the pairs at a cap the solver names) or `Row::SetupSearch(query)` (the setups for
  a target the solver names); the IR draft has no `derive`, so the derived state is read as that engine row, for the
  cap or the target. `class_covers` lets such a row cover the entry's `Row::Read(query)`. Every read of the query,
  also one the class lists as `Row::Read`, follows the exception's rules, each refused with
  `LG3:capability.not_granted`: the read is in the entry's own body, for a parameter of the entry as given (a read
  with no argument, for a literal or for anything computed is refused at the read, and a function the entry reaches
  that reads the query at `entry`; BND.2, BND.64); and its value is only delivered as read: returned, published as a
  view, or bound by `let` to a name used only so. Any other use is refused at the use (an operation such as `index`
  or `length`, a call, a field, a record or a collection, a condition or a match, a write, a proposal, a closure's
  result, a loop's count, initial value or body result), so the list keeps the engine's neutral order, marks no row
  best, and no entry tests a list for emptiness to read another cap; the solver chooses among the setups (BND.1). A
  branch delivers to the result only when its condition reads nothing but literals and the entry's parameters as
  given, and nothing is delivered where the program chooses whether it runs: under a branch whose condition reads
  more, at any depth, after a short-circuit `and` or `or` operand that does, in a loop's body whose count does, in a
  closure's body or in a child scope. So the program chooses neither the cap, also not among caps the solver named,
  nor whether to show the list (BND.2, BND.64); a branch inside a view's or a `let`'s value is a use. A value so
  delivered is the exception's output, so the shown-facts rule does not refuse its words. Previews keep their rules:
  an entry that reads the list or the setups prepares no preview. A class that lists one query as both exceptions is
  refused. `c600-runtime`'s codec carries the two rows as class-row tags 7 and 8.
  Outside the checker: the engine rows' neutral order and buffer-fixing setups, the empty list's wording (BND.2), the
  shell's keys and the solver's edit of a setup, and a job that names its cap through the command layer (KN5; LP2's
  pull request 326, `c600-command/tests/repair_list.rs`) (`c600-check/tests/exceptions.rs`: CT-BND-17's and
  CT-BND-18's checker halves). CT-BND-17 runs whole with LP3's C8 halves (the LP3 plan, section 9) and CT-BND-18
  in `c600-tools/tests/ct_bnd_18.rs`; `exceptions.rs` keeps their checker halves.
- Trace keys (LP2 draft until the IR freeze; LB1.10, LG2.23; LP3's plan, section 2). `trace_key` (`Prim::TraceKey`)
  gives a `TraceEqualityKey` (`Ty::TraceEqualityKey`), the key `trace-key/1` of a Full word or a menu word under the
  independence relation the run pins (`full-disjoint-support/1` or `jumble-locality/1`). Like a word equality key, a
  trace key is never literal letters: a proposal, word operation, word slot or result that takes one is
  `LG2:type.word_key_not_executable`, not a mismatch. It holds its word's letters for the built-in column, and it is
  no portable data, so a goal, an evidence sink or a snapshot read's argument that holds one is `LG2:type.mismatch`
  (`c600-check/tests/mismatch.rs`, `exactness.rs` and `built_in.rs`; `c600-syntax/tests/conformance_lg2.rs`:
  CT-LB1-06's language half, part).
- Generators (LP2 draft until the IR freeze; DET.61; LP3's plan, section 2). `rng_init` (`Expr::RngInit`) starts a
  `Generator` (`Ty::Generator`) on the run's seed, at the run's stream path followed by its literal segments, none or
  more, and `rng_draw` (`Prim::RngDraw`) draws its next block as a `Nat`. Both have the `random` effect and need the
  randomness grant, as `random_draw` does (CT-LG3-04's table). A generator stays a bare value, so that each cursor
  has one owner and only DET.61's alias copies it: the checker refuses the program when a record, a collection, a
  closure's capture or a host read's or write's type would hold one, or a declared type would hold one other than as
  a signature's own parameter or result (also a callable's), and when a view or a file would receive one. An exact
  equality, a type parameter or a host read's argument that takes one is `LG2:type.mismatch`
  (`c600-syntax/tests/generators.rs`).
- Child scopes (LP2 draft until the IR freeze; DET.60; LP3's plan, section 2). `child` (`Expr::Child`) runs its body
  under requested caps (`ChildCaps`: steps, kernel cost, memory and output, each a literal natural or omitted) and,
  when the body stops on a cap or an error of its own, a stop branch in the parent, with `status`, when it names one,
  a `Status` bound to the outcome. A child needs the `budget` effect and no grant. Its body borrows the names in
  scope but no display data, and it evaluates no display, reads no display accounting or clock and calls no function
  value, so exact inputs alone fix its budget use and its outcome (LG2.20; a draft rule). The stop branch is checked
  as the parent's code (`c600-syntax/tests/child_scopes.rs`).
- Loops (LP2 draft until the IR freeze; DET.55, LG2.20; LP3's plan, section 2). `repeat` (`Expr::Repeat`) runs its
  body `count` times, each time with `acc` bound to the value so far, from `init`, and its value is the last one. The
  count is an exact natural: an approximate number, display data, a clock reading or a budget observation there is
  `LG2:type.exactness_violation`, since loop bounds accept exact inputs only (LG2.20). The initial value and the
  body's result fit the loop's declared type, which holds display data only when it says so, and `acc` is bound in
  the body only. In the built-in column the value so far is no word as given, also under the name of the solver's
  word (LB3.5) (`c600-syntax/tests/loops.rs`, `c600-check/tests/built_in.rs`).
- A chain of `let`s is checked in a loop, so a long straight-line body does not deepen the checker's recursion, and a
  finding inside the chain keeps its nested path (`c600-check/tests/let_chains.rs`).
- Stage modules (LP2 draft until the IR freeze; RPI.68 to RPI.71, RPI.75, LG2.24, LG3.17, LG5.12). A module has a
  kind (`Module::kind`): a program module, or a stage (`ModuleKind::Stage`) with its kind, `vertex` or `fragment`,
  its declared `ops` and `overdraw` and its array inputs with their lengths. In source a stage's header is
  `stage vertex lens ops 16 overdraw 1 inputs [knots: 4];` in place of `module ... requires [...];`, and the eleven
  stage operations (`Prim::Stage`: `approx_sub`, `approx_mul`, `approx_div`, `approx_neg`, `approx_abs`,
  `approx_sqrt`, `approx_sin`, `approx_cos`, `approx_exp2`, `approx_log2` and `exclude`) lower only there. `check`
  applies the stage checker's structural rules to every stage module, alike under every profile, and the ordinary
  checker walks no stage body: binary32 only, with literals that are exact binary32 values; no capability; a plain
  `fn` with no effect; `index` of a declared array at a literal position below its length; `repeat` with a literal
  count; calls of stage functions of the same kind only, and no recursion; `exclude` only in a vertex stage, with a
  positive literal radius; and no `if`, `match`, closure, collection, clock, file, randomness, budget, snapshot,
  workspace, preview or view. Each refusal is `LG2:type.stage_rule_violation`, apart from a base-type mismatch
  (`LG2:type.mismatch`). A program that calls a stage function or holds one as a value is refused there, `admit` and
  `admit_built_in` refuse a stage function as a run's entry, and the interpreter loads no stage module.
  `admit_stage(program, module, entry, limits)` alone makes a `CheckedStage`: it adds the entry's rules (exported,
  takes every declared array, returns `record { x, y, z }` in a vertex stage and `record { r, g, b }` in a fragment
  stage) and the host's `StageLimits` (enabled tiers, RM.38's weights, the loop count and nesting bounds, and the
  `ops` and `overdraw` bounds, with the declared `ops` at least the weighted count of the expanded chain). The weights
  and bounds are test values until RM.38 is measured (RPI.79). Not built here: the raster bound, the ID pass, the host
  statuses, quarantine and compilation (RPI.72, RPI.75 to RPI.78), which are the adapter's and the host's
  (`c600-check/tests/stages.rs`: CT-LG3-10 whole and parts of CT-LG2-15; `c600-syntax/tests/stages.rs`: parts of
  CT-LG5-08).
- Evidence kind: source and fixture. A pass is not Windows/DirectX, input, long-session or performance evidence.

## Status of M7: the parser

The syntax details below are drafts, settled under the plan's section 8. They freeze with the syntax and the IR
(PHS.49) and may change until then.

- Edition and bytes (LG1.1 to LG1.3). Every file starts with `edition "6cs-draft-1";`, the only supported edition. A
  missing header gives `LG1:syntax.edition_missing` at an insertion point, another edition gives
  `LG1:syntax.edition_unsupported`, and nothing after the header is read. Source is strict UTF-8 with LF line ends and
  no byte-order mark; anything else is `LG1:syntax.invalid_encoding` with no span, the digest of the raw input and the
  offending byte interval (CT-ERR-05). Digests are SHA-256, from `c600-digest`.
- Grammar. LX-8's candidate A
  ([description](../work/experiments/language-lx/lx8-surfaces/candidate-a/DESCRIPTION.md)), with these additions
  for the IR draft: `export` before `fn`, `view` or `command`; generic parameters `fn f<T,E>(...)`, which name types
  and effect variables; closures `fn(x: T) -> R { ... }`; records `record { a = e }`, field access `e.a` and calls of
  any expression (`values().ratio`); type arguments in calls of reserved operations (`read_snapshot<Colour>("q")`);
  the types `Display<T>`, `Approx<P24>` (also `P53`, `P113`), `record { a: T }` and `fn(T) -> R effects [...]`;
  literal arms and a final `_` arm in `match` beside LX-8's binder arms; and the literals `true`, `false` and `()`.
  `charge` and the command metadata are optional; LX-8 required them. `true`, `false` and `_` are no longer names.
- Reserved names. The keywords (`KEYWORDS` in `c600-syntax/src/lower.rs`) and the reserved operations (`BUILTINS`)
  cannot be bound. A reserved operation is called like a function: `if(c, a, b)` and `display_if(c, a, b)` in LG2.20's
  notation, the exact and approximate operations, the effect operations and the accounting reads. `tag`, `colour`
  and `target` take a literal string and give a literal.
- Names. A name is bound once in a function: a `let` or a parameter does not shadow another binding, a module
  function or an import alias. An import names a module by the SHA-256 of its source file, among the files given to
  one `lower` call, and the module's functions are written `alias.f`. Another module calls or names only a module's
  exported functions, and a call of a module function, like a call of a reserved operation, gives one argument per
  parameter; lowering refuses either fault at its source text. The module loader, with its lockfile and its LG5
  refusals, is not built.
- Effects and capabilities. A declaration's `effects [...]` lists effects (`snapshot`, `view`, `workspace`,
  `preview`, `budget`; `file` and `random` also name their capability), capabilities (`file`, `clock`, `random`,
  `network`, `process`, `session_file`) and generic effect variables. A module's `requires [...]` lists capabilities
  only; an effect there is refused (LG3.8, LG3.9).
- Exact literals (LG1.6, CT-LG1-05). Integers of any length, rationals with a nonzero denominator and the sign on the
  numerator, Q(√5) pairs and words keep exact decimal text in the IR; `~53"0.125"` keeps its decimal text and its
  precision. A rational is not reduced to lowest terms until M2 brings big integers, so `6/8` and `3/4` are
  different literals for now.
- Exact numbers in the checker. `exact_add`, `exact_less` and `exact_eq` take `Rat` and `Q5` values too, and `Nat` and
  `Int` widen to `Rat` and `Rat` to `Q5` (Z ⊂ Q ⊂ Q(√5)) in operands, branches, arguments and results. A `Budget` is
  a cost and mixes with integers only (LG2.6): an operand that mixes it with a `Rat` or `Q5` value, like a wider result
  in a narrower slot, is `LG2:type.mismatch`. Running this arithmetic and its tariffs are KN3's and come with M2.
- Recovery (CT-LG1-04). A damaged declaration becomes a recovery hole with `LG1:syntax.recovery_gap`, and parsing
  resumes at the next `fn`, `view`, `command`, `import` or `export` in column 1. A draft with a hole has no layout and
  does not lower.
- Lexing stays linear. A quote that opens no string on its line is scanned to the line's end once, and the quotes that
  scan stepped over are not scanned again, since they cannot open a string either; the tokens are unchanged (lead L5
  of the M7 attack, 16 November 2026).
- Canonical layout (LG1.4, CT-LG1-03). `format` depends on the parsed program only: four-space indents, a blank line
  before each declaration, imports directly after the module line, one `let` or `return` per line, `match` arms and
  closure bodies on lines of their own, and type arguments and generic parameters without spaces (`<T,E>`, as LX-8's
  type strings). The layout drops comments, as in candidate A. Since LP4-1 the parse keeps them, and whitespace, as
  trivia for the authoring services (PHS.21), but no layout keeps them yet.
  `c600-syntax/tests/fixtures/canonical.6cs` pins one expected output by its SHA-256.
- Source map (LG4.12). `lower` returns, for each module, the source digest, each function's declaration range and
  the source range of every IR path in each body, so a checker diagnostic's function and path map to source text.
- What parses but does not lower yet. Lowering refuses each of these with a message naming what it waits for: a
  declared `charge` (LG4.8's tariffs), command metadata (the command table, LP3), binder arms and LX-8's domain types
  such as `Word<Full,Retained>` (the IR's domain families, M4 and M5), and LX-8's fixture menu letters
  `menu[2:spin]` (a letter of the pinned menu is `cap:item`, below). Type arguments on a program's own function are
  refused: the IR draft has no explicit instantiation.
- Lists and words (LP2 drafts). `List<T>` and `WordEqualityKey` are types, and a `List` of a function is refused at
  lowering. `list<T>(a, b, ...)` builds a list; `length`, `index`, `push`, `concat`, `inverse`, `conjugate`,
  `commutator`, `compose` and `word_key` are reserved operations called like functions
  (`c600-syntax/tests/collections.rs`).
- Sets and maps (LP2 drafts). `Set<T>` and `Map<K, V>` are types; a set element or a map key that is not exact in its
  type is refused at lowering. `set<T>(a, ...)` and `map<K, V>(k, v, ...)` build them; `contains`, `insert`,
  `lookup`, `elements`, `keys` and `entries` are reserved, and a listing names its order as its second argument,
  `ascending` or `descending`, which are reserved words.
- Compressed words (LP2 drafts). `CompressedWord` is a type, refused as a set element or a map key at lowering;
  `compress` and `expand` are reserved operations called like functions.
- F2 and F5 (LP2 drafts). `F2` and `F5` are base types; `to_f2`, `to_f5`, `residue`, `field_add`, `field_sub`,
  `field_neg`, `field_mul` and `field_inverse` are reserved operations called like functions, and no literal is an
  element (`c600-syntax/tests/fields.rs`).
- Snapshot reads with an argument (LP2 draft). `read_snapshot<T>("q", x)` lowers to the read with an argument;
  `read_snapshot<T>("q")` stays the literal read.
- Menu words (LP2 draft). `MenuWord` is a type, and `menu[cap:item, ...]` is a word of the run's pinned menu: each
  letter two naturals below 600, counted from 0 and laid out in canonical decimals. A letter out of range is refused
  at its source text (`c600-syntax/tests/menu_words.rs`).
- End tests' verdicts (LP2 draft). `RetainedCheckpointVerdict`, `LabelCheckpointVerdict` and `StickerTestVerdict`
  are types; one as a set element or a map key, alone or in a record, is refused at lowering
  (`c600-syntax/tests/end_tests.rs`).
- Trace keys (LP2 draft). `TraceEqualityKey` is a type, and `trace_key` is a reserved operation called like a
  function (`c600-syntax/tests/conformance_lg2.rs`).
- Generators (LP2 draft). `Generator` is a type, and `rng_init` and `rng_draw` are reserved operations called like
  functions; `rng_init`'s arguments are text literals, its stream path's segments, none or more
  (`c600-syntax/tests/generators.rs`).
- Child scopes (LP2 draft). `child(caps, body, stop branch)` is a reserved operation called like a function: `caps`
  is a record of literal naturals named `steps`, `kernel_cost`, `memory` and `output`, each at most once and any of
  them omitted, `body` a closure without parameters, and the stop branch a closure with the body's result type and
  no parameter or one `Status` parameter. Another shape is refused at lowering (`c600-syntax/tests/child_scopes.rs`).
- Loops (LP2 draft). `repeat(n, init, fn(acc: T) -> T { ... })` is a reserved operation called like a function: its
  body is a closure with one parameter of the loop's result type, bound in the body only, and another shape is
  refused at lowering (`c600-syntax/tests/loops.rs`).
- Exact products and assertions (LP2 drafts). `exact_mul(a, b)` multiplies two exact numbers, widened as
  `exact_add`'s operands are; a `Budget` operand is `LG2:type.mismatch`, since the draft multiplies numbers, not
  costs. `assert(p)` is `Unit`, and its predicate is an exact `Bool`, as an `if` condition is (LG2.20)
  (`c600-check/tests/mismatch.rs`, `c600-check/tests/exactness.rs`). `q5_parts(x)` takes a number up to `Q5` and
  gives `record { a: Int, b: Int, d: Int }`, the canonical triple of x = (a + b√5)/d in lowest terms with d > 0
  (ADR-L03.6, PS1.4); `q5_of(a, b, d)` takes three integers and gives a `Q5`. A `Budget` operand is
  `LG2:type.mismatch`, and a stage module refuses both (RPI.70; LP2's pull request 340).
- Not run or not applicable, which is no pass (PHS.63): CT-LG1-01's second half, mixed dependency editions, while one
  edition exists; CT-RTE-18 (LQ13 was answered without it).
- Evidence kind: source and fixture. A pass is not Windows/DirectX, input, long-session or performance evidence.

## Status of M2: the interpreter and the meter

- The reference meter of chapter 09 (DET.55 to DET.60) replays the 40 frozen contract-vector traces with every field
  compared exactly, and the meter boundary vectors derived from them (`c600-run/tests/traces.rs`). The frozen test
  tariffs are `Tariffs::TEST`, labelled test-only.
- `c600-run/tests/det65_programs.rs` runs 32 of the 40 traces as programs through `Loaded::run`: nested-calls,
  equal-to-limit, step-precedence, simultaneous-exhaustion, simultaneous-memory-output, the four generator traces
  generator, generator-alias-cursor, generator-returned-owner and generator-child-paths, the three display traces,
  output-equality, output-retained-at-cutoff, the three exact-multiplication traces, alias-release, alias-still-live,
  pointwise-failure, final-loop-test, the five child traces child-allowances, child-return-owner,
  root-before-child, dimension-before-scope and nested-child-ledger, child-decode-error-releases-reservation, whose
  child decodes bytes the code holds (`text_decode_hex`, pull request 367), the two decoding traces decode-scalar
  and decode-surrogate, on bytes the host delivers as a snapshot, and the three streaming traces streaming,
  streaming-complete-split and streaming-complete-coalesced, one program for both transport partitions. Each frozen
  row, each cutoff or stop (dimension or phase, scope, denied row, source range) and each child record equals the
  vector, under LP3's limit mapping (`c600-runtime/tests/ct_rte_04.rs`): CT-DET-26, 27, 28 (synthetic), 29, 30, 31 and
  32, all in part, on RT1 only. The alias traces cross a call, since `let` scopes nest. A child trace that completes
  takes its limits at its first `child` row, so that each requested cap is cut as the trace's is.
- The other 8 are listed with the reason the IR draft cannot express them: a call frame without a slot that holds two
  bindings (1, generator-nested-same-name), text order (1), a flow check (1, abstract-trace-charges) and display form 2
  (5). They still replay through the meter.
- The product output profile is not the fixtures' (CV.5): a realisation publishes values with the fixture's byte
  counts, and `output-retained-at-cutoff`'s one-byte denied unit gets two bytes of kernel-cost headroom, since no
  `c600-json-1` unit is shorter than three bytes.
- `load` checks the IR identity and the program's names, arities, literals and precisions
  (`LG4:input.ir_version_unsupported`, `LG4:input.ir_rejected`). `Loaded::run` evaluates with an explicit work stack:
  one charge row per operation (DET.55), interruption observed before each operation (DET.56), aliases and owners
  (DET.58), each output unit as one fused row of encoding work, buffer and bytes (DET.56, DET.59), a preview
  request's priced `encode` pass before `invoke`, the generator `sha256-counter-draft-1` (DET.61) and `display_if`
  charging the selected branch only (DET.64). A stop names the call path, the operation and its source range.
- Exact products and assertions (LP2 drafts). An integer `exact_mul` charges the tariffs' integer product: under
  `Tariffs::TEST` one kernel unit per pair of eight-bit limbs, and a reservation of one byte plus the operands' bits
  in bytes (DET.57). A `Rat` or `Q5` product charges one unit per pair of the operands' widened bytes and reserves
  what the unreduced product could need; the reduced product settles the reservation. `assert` is one step with no
  memory after its predicate (DET.55): a false predicate ends the run with `LG4:input.pointwise_failed` in the effect
  phase, and the cleanup releases what the run held (`c600-run/tests/interp.rs`, `conformance_det.rs`,
  `det65_programs.rs`). `q5_parts` charges one unit per widened operand byte and reserves its record exactly;
  `q5_of` is priced as the product of a + b√5 and 1/d and settles on the canonical value, and a zero denominator
  stops at its `assert` with `LG4:input.pointwise_failed` before the construction (DET.57; LP2's pull request
  340, `c600-run/tests/q5.rs`).
- Whole-state contracts (LP2 draft until the IR freeze; LG4.10, LB1.7). A function of kind `FnKind::Contract`, in
  source `contract settled() -> Bool effects [snapshot] { ... }`, takes no parameter, returns an exact `Bool` and
  needs nothing beyond reading the snapshot and child budgets, so its result is a function of its revision; the checker
  refuses another shape (`LG2:type.mismatch`, `LG3:effect.undeclared`) and any call or reference to a contract, also
  from a loop's closure or another contract (`LG3:effect.undeclared`), so no call or loop runs one per call.
  `contract::Revisions` runs a contract once per revision: asked again at its revision it gives the kept run and asks
  the host nothing; a changed revision or program digest runs it again and replaces the kept run. A kept run is a
  value, never evidence. The agreement-test form is any test that runs a contract (`c600-run/tests/contracts.rs`:
  CT-LG4-05 whole).
- A language evidence builder and its build cache (LP2 draft until the IR freeze; DET.44 to DET.46), in the crate
  `c600-build`, outside the interpreter. `Grant` takes an output and a scratch directory and refuses either with
  `LG3:capability.grant_mismatch` when it is the source tree, lies inside it or holds it. `build` runs an exported entry
  of a sealed module closure (the program's modules must be exactly the sealed ones; an import resolves only among the
  given sources) on the caller's thread, with the spec's seed. Its host grants no clock, snapshot, declassifier,
  preview, view or workspace (`LG3:capability.not_granted`); it reads only the captured inputs, by logical name and
  type, and writes only files at plain relative paths under the output grant. The cache lives under the scratch grant,
  keyed by the SHA-256 of the IR and execution contract, the sealed modules, the entry, the arguments, the seed, the
  captured inputs, the limits and the tariffs; the locations enter no key. Before reuse the host checks the entry's key,
  its recorded inputs, its result digest and every stored file; after a construction it captures the inputs again. A
  failed check gives the host-envelope diagnostic `LG4:input.cache_entry_mismatch`: a rejected entry is a miss rebuilt
  with the original charges (DET.16), and an input changed during construction leaves nothing stored or published. Only
  a completed run is stored. The checks catch a changed entry; they authenticate nothing. DET.47's atomic publication is
  chapter 05's (CT-DET-23 not run). `c600-build/tests/builder.rs`: CT-DET-21 and CT-DET-22 whole.
- Generators (LP2 drafts; DET.61). `rng_init` is one step priced at the input-byte tariff per byte of the hashed key
  input (13 bytes of domain, the seed's 32, 4 for the segment count and per segment 4 and its bytes), and it holds a
  generator's 41 units. Its stream path is the run's assigned path followed by its segments, on the run's seed, so
  re-initialising a path reproduces that stream, as DET.61 says. Without a seed it charges its step alone and stops
  with `LG3:capability.not_granted`, as `random_draw` does. `rng_draw` draws the cursor's next block, one step at the
  block tariff, holding a 256-bit natural's 33 units. A borrowed argument draws from its owner's cursor and a move
  hands the cursor on; an alias (a borrowed generator bound to a name or returned) copies it at 41 units, so the copy
  repeats the original's blocks. A generator has no canonical form, so an entry neither takes nor returns one
  (`PS1:input.noncanonical_value`). The run's own generator still starts at `entry#rng_init` whenever the run has a
  seed; in a child scope the path also names the child's invocation (below) (`c600-run/tests/generators.rs`,
  `det65_programs.rs`).
- Child scopes (LP2 drafts; DET.60, DET.61). `child` costs one step in every enclosing scope. Its body then runs in
  a scope of its own under the effective caps: each requested cap cut to what every enclosing scope has left, an
  omitted cap being that remainder. What the child charges counts in its enclosing scopes too; the body's `call` and
  `return` are charged in the child, its frame borrows the enclosing frame's slots without charge, and its result
  moves to the parent. A stop of the child's own scope, a cut-off or a domain error, unwinds the child: its owners
  and reservations go uncharged, and its stop branch runs in the parent, after a four-byte `status` row when it
  names a status. A limit of an enclosing scope stops that scope, the root's the whole run, and an interruption
  stops the whole run; an outer child's cap comes before an inner one's, and DET.56's dimension order before the
  scope order. The summary records each child with its requested and effective caps, its use, its outcome and stop,
  and the memory it returned. An `rng_init` in a child streams from the run's path followed by the child's
  invocation index under its parent, counted from 0 per parent (DET.61). A child borrows its parent's generators as
  any parent input, so its draws advance their owners' cursors, and a cut-off rolls none back
  (`c600-run/tests/child_scopes.rs`, `det65_programs.rs`).
- Byte decoding (LP2 drafts; DET.62). `Bytes` holds captured octets, from a `bytes("…")` literal of hex digit pairs
  or a host result; it has an exact equality but no canonical form and no key order, so no entry, output, goal,
  evidence, set element or map key holds it. `text_decode` is one step at the input-byte tariff per byte that
  reserves 4 bytes per byte before reading one and settles to the text's 4 bytes per scalar; invalid UTF-8 or a
  surrogate stops the run, or a child alone, with `LG4:input.text_scalar_invalid` in the effect phase, nothing bound.
  The wire codec carries bytes as value tag 25 and type tag 28 (`c600-run/tests/decode.rs`, `det65_programs.rs`).
- Streamed words (LP2 drafts; KN2.10, LG4.6, DET.63). `word_stream(c, k)` streams a compressed word in chunks of the
  exact size `k`: a `word_open` row of one step at `stream_setup` that reserves the cursor (the length's integer
  size), then one `chunk:<position>` row per chunk at `expand_per_letter` and `replay_per_letter` per letter, which
  preflights its share and 2 bytes of scratch per letter before it expands one and releases the scratch in the same
  row. The schedule comes from the length and `k` alone; a final short chunk has its own share; a `k` of 0 stops
  the run with `LG4:input.pointwise_failed` before any step. Expansion keeps the compressed structure and a stack
  of parts, never the whole word, and an interruption stops the stream at its open or a chunk boundary. The
  result, `WordStream`, holds the length and the literal digest, which `stream_digest` reads as 64 hex digits:
  the kernel's, the same for every chunk size. A stream has no equality, key order or canonical form and stays in
  its run; the wire codec gives its type tag 29 and refuses its value (`c600-run/tests/streams.rs`,
  `det65_programs.rs`; CT-LG4-03 and CT-DET-30 in part).
- Loops (LP2 drafts; DET.55, DET.56, DET.58). A loop runs in its function's frame, with its value so far in a slot
  of the frame. The initial value fills the slot as a `let` binds: an owned value with a `visit`, a borrowed one
  with an `alias` of its size. Each loop test is a one-step row `loop_test:k`, the final false one included. A row of
  iteration k is located below it, `label:path#body:k > `, and a locator names the call sites and iterations that
  enclose a row in the order they were entered. At the end of iteration k the body's result refills the slot, in a
  row of the iteration: the slot itself with a `visit` that releases nothing, another owned value with a `visit` and
  a borrowed one with an `alias`, each releasing the value it replaces in the same row, after admission (DET.56). The
  count is held until the loop's `scope_exit`, which releases it, and the value so far then moves out as the loop's
  value. A cut-off in an iteration unwinds the loop as any frame. A count that reaches the run unchecked and is no
  natural is refused at `loop_test:0`, after the initial value fills the slot, with `LG4:input.ir_rejected`, and a
  count beyond 128 bits runs as 2^128 - 1 iterations, more than any step budget allows (`c600-run/tests/loops.rs`,
  `det65_programs.rs`).
- Values: exact `Int`, `Rat` and Q(√5) arithmetic, order and equality with widening, on `num-bigint`; `Approx<p>` for
  p in {24, 53, 113}, rounded to nearest with ties to even; canonical `c600-json-1` bytes with FP.6's exact and `f64`
  forms. Deep values compare, encode and drop without recursion. A word literal's letters are generators, of
  magnitude 1 to 1200 (chapter 01), and a literal with another letter is refused. No alphabet has a letter of
  magnitude above 1200, so a word with one, also inside an aggregate, has no canonical form, and `Loaded::run` refuses
  an argument that holds one with `PS1:input.noncanonical_value` before the run starts (`c600-run/tests/interp.rs`).
  A letter of 0 still runs: the kernel refuses it as a generator when it acts, and `c600-jumble-tool` reads a `Word`
  as flattened grip and item indices, which start at 0, until it takes menu words (an LP2 draft, below).
- Drafts, which freeze with the IR (PHS.49): operation names, the stop locator, the frame layout (parameters,
  captures, `let` and loop slots and operand temporaries; a body's own result has no slot), a value's canonical byte
  length as retained metadata (so an output unit is one fused row), the receipt request record, the JSON form of a
  word (a word key has none, LG2.13; LG2.18's key contract stays an open choice, PHS.55) and `DraftTariffs`, test-only
  tariffs for the rows the frozen vectors do not price. The IR draft has one read construct for snapshots and
  workspace records; the IR freeze should split it.
- Lists and words (LP2 drafts): `Value::List` (`Value::list`) and `Value::WordEqualityKey`. A list's logical size is
  a record's, after DET.57's sequence row: a 4-byte tag, 8 bytes per variable-sequence element, and the elements. Its
  JSON form is an array in written order. Each operation is one step: `length` reserves its integer; `index` lends or
  retains an element as a field read does, and an index outside the list stops with `LG4:input.pointwise_failed`
  before its step; the builders are priced before they build, `expand_per_letter` per letter built or scanned and
  `LIST_PER_ELEMENT` (1) per element built. These prices are drafts, since DET.55 names no row for them. The key is
  LG2.18's Full free reduction and has no output form, so a run that would emit one stops with
  `PS1:input.noncanonical_value`. Tests: `c600-run/tests/collections.rs` (CT-LG2-12's run-time language half and
  CT-LG4-01 for lists and words, parts) and the key halves of CT-LG2-11 and CT-LG2-13 in `conformance_lg2.rs`.
- Sets and maps (LP2 drafts): `Value::Set` and `Value::Map` (`Value::set`, `Value::map`) keep their elements, or
  their `{ key, value }` entries, in the canonical key order `value::key_order` (DET.9): exact numbers by value,
  texts by scalar values, words by letters, word keys by LG2.23's token order (the generator, then the sign, the
  inverse first), aggregates part by part, kinds by a fixed rank, and for equal numbers in
  different representations the explicit secondary key `Int`, `Rat`, `Q5`. Equal keys are one key; the lowest
  representation stays, and a map's later value replaces an earlier one. They are sized as lists and encode as JSON
  arrays in that order; a host value out of that order has no canonical form. Draft prices: `set` and `map` are one
  step at `LIST_PER_ELEMENT` per element and n·⌈log₂ n⌉ comparisons of the widest key, each `compare_per_unit` per
  unit of two keys; `contains`, `insert` and `lookup` cost ⌈log₂(n + 1)⌉ comparisons of the probe, an insertion adds
  `LIST_PER_ELEMENT` per element built, and `elements`, `keys` and `entries` the same per element listed. A missing
  key stops `lookup` with `LG4:input.pointwise_failed` before its step. Tests: `c600-run/tests/collections.rs`
  (CT-LG4-01 for sets and maps and CT-DET-05's insertion-order half, parts).
- Compressed words (LP2 drafts): `Value::Compressed` holds a `value::Compressed`, a straight-line program whose parts
  are leaves of literal letters or concatenations of two earlier parts, with its exact length. A join of two equal
  values, compared by value, adds one part, so 70 doublings of three letters keep 71 parts for 3·2⁷⁰ letters; an
  empty operand gives the other. Its logical size is a 4-byte tag and, per part, its payload (a leaf 4 + 8 + 2 bytes
  per letter, a concatenation 20) and its exact length as an integer, which a reading needs to find a position; in a
  container it is a variable sequence. So a chain of n doublings is charged for the about n²/2 bits of lengths it
  keeps. A join shares its operands' stored lengths instead of copying them, so its work is one step per part, and
  each word still counts them in full (DET.57). Draft prices: `compress`
  at `expand_per_letter` per letter; `concat` at the comparison of the two operands, the integer addition of their
  lengths and `LIST_PER_ELEMENT` per part built, reserving the size of a join of two different words; `length`
  reserves its integer; `expand` at `expand_per_letter` per letter and `LIST_PER_ELEMENT` per part it may visit,
  2·(height + count) + 1, reserving 2 bytes per letter, and a range past the end stops it with
  `LG4:input.pointwise_failed` before its step. A compressed word has no canonical form, so a run that would emit one
  stops, and an argument that holds one is refused, with `PS1:input.noncanonical_value`. An exact equality with a
  compressed operand is refused as malformed IR; one held in a record or a collection only the checker refuses.
  `part_list` gives the parts in order and `from_parts` rebuilds the word from them, for a transfer form, deriving its
  lengths, heights and size. It refuses a list without parts, a letter outside 1 to 1200, a concatenation of a part
  not before it or of an empty part, and a part the word does not reach; it accepts any other straight-line program,
  also a shape `concat` does not build. The derived lengths reach about n²/2 bits for a chain of n doublings, so a
  decoder of untrusted input bounds the parts.
  Tests: `c600-run/tests/collections.rs` (CT-LG2-14's language half, part).
- Not built for compressed words: their inverse, conjugate, commutator and composition, `word_key`, an exact equality
  and an output form; a stream fed chunk by chunk into the kernel's replay, since a run proposes no stream
  (CT-LG4-03's remaining part).
- F2 and F5 (LP2 drafts): `Value::Field` holds the field and a residue below its prime, and `Value::field` reduces an
  integer of any size. Draft prices, one step each and charged before the work, with a mistyped operand rejected as
  malformed IR before any charge: `to_f2` and `to_f5` at the integer row's price for adding the prime, one walk over
  the integer's limbs, so a reduction is priced by the integer's size (DET.55: no unmetered big-integer operation),
  reserving one byte; `field_add`, `field_sub`, `field_neg` and `field_mul` at no kernel units, reserving one byte;
  `residue` reserves its integer. `field_inverse` is guarded by a pointwise assertion (DET.55): the predicate
  `field_nonzero` reserves its `Bool`, the `assert` step consumes it, and the inverse follows. The inverse of zero
  stops at the `assert` step with `LG4:input.pointwise_failed` and keeps the admitted charges; chapter 13 has no
  arithmetic domain code, so the code is a draft. In the key order an element is the last kind, by its field, F2
  first, then its residue, and the equality of two elements costs no kernel units. An element has no canonical form,
  so a run that would emit one or write one to the workspace, a view or a file stops before the host is asked, and an
  argument that holds one is refused, with `PS1:input.noncanonical_value`; the checker admits such a write, as it
  admits one of a compressed word, and a program outputs or stores the residue. Tests: `c600-run/tests/fields.rs`.
- Snapshot reads with an argument (LP2 draft): the argument's canonical bytes are priced in DET.59's `encode` row
  before the `kernel` row, as a preview request's are, and the kernel row counts them as output. An argument with
  no canonical form stops with `PS1:input.noncanonical_value` and an approximate one with `LG4:input.ir_rejected`,
  both before the host is asked. The host gets `Request::SnapshotWith { query, arg, ty }` with `arg` the canonical
  bytes, which `provenance::parse` reads; a literal read stays `Request::Snapshot` (`c600-run/tests/snapshot_arg.rs`).
- Menu words (LP2 draft; LB1.8, LB1.9): `Value::MenuWord` holds letters `[cap, item]` and the jumbling identity block
  of their alphabet (`menu::MenuIdentity`, KN0.1, its fields in KN1.4's order). A run pins at most one alphabet,
  `Options::menu` (`menu::MenuAlphabet`: the identity block, the cap count and the inverse item of each item, an
  involution), and every menu word in the run is a word of it. Loading refuses a literal with an index of 600 or
  more, as it refuses a Full word's letter above 1200. A literal takes the pinned block when it is read; a letter
  outside the alphabet, or a literal in a run that pins none, stops the run with `LB1:input.incompatible_alphabet`.
  `Loaded::run` refuses with that code an argument that holds a menu word of another alphabet or with a letter
  outside it (one whose block names no alphabet with `PS1:input.noncanonical_value`), and a host answer that holds
  one stops the run after its kernel row. An inverse reverses the letters and replaces each item with its inverse item;
  the operations are priced as the Full word operations, `expand_per_letter` per letter built, and a menu word's size is
  4 bytes per letter. Its JSON form is `{"letters":[[cap,item],...],"menu":{...}}` with the block's fields in key order.
  A match compares menu words by their letters. A host previews one through
  `Request::ProposeMenu { letters, menu, target, request }`, a request of its own, so a host that knows only
  `Request::Propose` refuses it; `Request::is_proposal` names both, and a host that gates previews gates both
  (`c600-run/tests/menu_words.rs`).
- End tests' verdicts (LP2 draft; LB3.2): `Value::EndTest(test, outcome)`, which a host answers for its test's
  row. A match arm's status tag matches the verdict of that outcome, and no other pattern matches a verdict. Its
  size is 2 bytes, and its JSON form is `{"outcome":"...","test":"..."}`, the outcome's tag and the test's row
  (`c600-run/tests/end_tests.rs`). Where the checker cannot see a verdict, behind a type variable or in an unchecked
  program, the run stops with `LG4:input.ir_rejected` before the step at an exact equality whose operand is or holds
  a verdict, and at a match on a verdict whose `_` arm takes `unestablished` together with another outcome.
- Trace keys (LP2 draft; LB1.10, LG2.23): `trace::Relation` is the independence relation `full-disjoint-support/1`
  of one pinned 600-cell-Full model, built from the 1,200 generators' complete supports
  (`Relation::full_disjoint_support`: two distinct generators are independent exactly when their supports are
  disjoint, whatever the signs, and a generator depends on itself) or from its wire parts (`Relation::from_parts`,
  which recomputes the digest). A run pins at most one, `Options::relation`. `trace_key` gives
  `Value::TraceEqualityKey`, a `trace::TraceKey`: the relation's header (puzzle, model ID, generator count, the key
  `trace-key/1`, the contract and the digest, in LG2.23's order) and the word's least topological order under
  LG2.23's token order, every occurrence kept, so nothing cancels or merges. It is priced before it is built,
  `expand_per_letter` per unit its scan reads (`Relation::reads`: each generator once; per letter one unit, a heap
  push and pop of 11 units each, and two units per generator its generator depends on), and its size is 2 bytes per
  letter. Without a pinned relation the run stops with `LB1:input.key_contract_mismatch`, and at a letter outside 1
  to 1200 with `LB1:input.incompatible_alphabet`. Keys compare by header, then token by token. A key has no
  canonical form, so an argument that holds one is refused, and a run that would emit one stops, with
  `PS1:input.noncanonical_value`; a host answer that holds one under another header, or whose letters are not their
  own key, stops the run after its kernel row with `LB1:input.key_contract_mismatch`. That check is not metered (open
  until the IR freeze): it does the work of `trace_key` on the answer's keys, which the row's result bound limits,
  since a key's size is 2 bytes per letter. `c600_tools::supports` reads the loaded model's supports from the kernel,
  and `full_relation` builds its relation once per model in a process.
  Tests: `c600-run/tests/trace_keys.rs` (CT-LB1-06's run-time half on synthetic supports, part) and
  `c600-tools/tests/supports.rs` (the relation of the model in `assets/`, and CT-LB1-06 on it).
- The jumbling trace key (LP2 draft; LB1.10, LG2.23). `c600_jumble::locality::Locality` regenerates
  `jumble-locality/1` from the pinned model and the exact Δ(c,d) = R²(|n|² + n_c·n_d) − 2κ² in Q(√5), with R² the
  largest squared norm of cap 0's region (96 − 32√5): distinct grips c and d are independent exactly when Δ(c,d) is
  at most zero. Every grip interacts with 80 others, antipodal grips are independent, and grip 0's pairs match stage
  14's exact shells (`research/jumbling/restoration/14-blocking-structure/results/locality-summary.json`).
  `Locality::bits` gives one bit per pair in ascending (c, d), the layout of `trace::Relation::bits`, and
  `trace::Relation::jumble_locality` builds the relation from them under the jumbling identity block and the menu's
  item count, so a caller declares no independence of its own. The digest covers the contract, the puzzle, the model
  ID, the grip count and the bits (the table is the model's); the header adds the menu's three identity fields and its
  item count. Letters at one grip are dependent, the identity item's too, and menu letters order by grip, then item.
  `trace_key` on a menu word needs a pinned jumbling relation that keys the run's menu alphabet
  (`Relation::keys_menu`: one identity block, grip count and item count); without one, and for a Full word under a
  jumbling relation, the run stops with `LB1:input.key_contract_mismatch` before the step. It is priced as a Full key
  (`Relation::menu_reads`, the grip in the generator's place), and its size is 4 bytes per letter. A host answer's
  menu key is checked as a Full key is. `c600-runtime`'s codec carries a menu key as value tag 26, with its header's
  identity block and item count, and a jumbling relation with the menu's three fields after the model ID and the item
  count after the grip count; the Full forms are unchanged. A jumbling host pins the relation (LP3: `c600-jumble-tool`).
  Tests: `c600-jumble/tests/locality.rs` (the table, and CT-LB1-06's jumbling half on the model, part),
  `c600-run/tests/menu_trace_keys.rs` (on synthetic bits) and `c600-runtime/src/codec.rs`'s tests.
- The jumbling word key (LP2 draft; LG2.18, LG2.23; LP2's pull request 338). `word_key` on a menu word gives
  `word-key/1` under the item matrices of the run's pinned menu (`c600_run::menu_key`): each maximal run of adjacent
  letters at one grip becomes one token, the grip and the exact product of the run's items in chronological action
  order, in the menu's base frame, so two tokens at one grip are equal exactly when their twists are. A token stays
  when its product is the identity, and a product outside the menu stays an exact group element; no commuting swap,
  identity deletion, minimality or dynamic-target reduction is part of the key. A key carries LG2.23's header (the
  alphabet's identity block, the grip count and the item count) and orders by header, then token by token. A run
  keys menu words only when its pinned alphabet carries the item matrices (`MenuAlphabet::with_matrices`, which
  refuses matrices that do not give the block's `menu_id` or `menu_order`), and such a run keys no Full word, so the
  keys a run compares share one header: a menu word's key without the matrices, a Full word's key in such a run and a
  host answer that holds a key under another header or out of its canonical form stop the run with
  `LB1:input.key_contract_mismatch`. A key has no output form, so no argument or result holds one (LG2.13), and a key
  where a menu word's letters are required is `LG2:type.word_key_not_executable`. The key reserves its products' size
  bound and charges each product by its operands' sizes (a draft DET.57 price). `c600-runtime`'s codec carries a menu
  word key as value tag 27 and a pinned menu alphabet with a matrices flag (the runtime's protocol, below). Tests:
  `c600-run/tests/menu_word_keys.rs` (CT-LB1-02's jumbling half on a synthetic menu of seven items, part),
  `c600-check/tests/mismatch.rs`, `c600-runtime/tests/protocol.rs` and the codec's own tests.
- The alphabet and relation a jumbling host pins: the alphabet from `c600_kernel::IdentityBlock` and the inverse items
  of its menu (`menu::MenuAlphabet::new`), with the menu's item matrices for word keys
  (`menu::MenuAlphabet::with_matrices`), and the relation from `c600_jumble::locality::Locality::bits` and
  `trace::Relation::jumble_locality`. LP3's C8 pins the alphabet with the matrices (`word::Alphabet::pinned`) and the
  relation (`locality`) in `c600-jumble-tool`, where CT-LB1-02's key half with its library entry `menu_word_keys` and
  "Word keys remain separate", CT-LB1-06's last part, run on M5's states (`tests/lb3.rs`, `tests/ct_lb1_06.rs`). Not
  built for trace keys: the operations on compressed words, which LP3 does not need, and EV4.9's report of a key's
  contract.
- Evidence kind: source and fixture.

## Status of M3: the run-time boundary

- `provenance` reads, checks and builds FP.17's provenance blocks: a strict `c600-json-1` reader, FP.5's canonical
  bytes and FP.17's validator as `check_vectors.py` has it. A block that does not read, is in another form or names a
  pin the policy in force does not hold gives `PS1:capability.provenance_incompatible`. FP.18's four example blocks
  digest as frozen, and the runtime rebuilds the two capability-form blocks byte for byte.
- `boundary::Boundary` is the runtime's host adapter for one run. The host builds the block of everything a run
  stores or proposes (BND.22), with the blocks the run took in as parts (BND.24); workspace records and granted files
  keep their blocks (BND.32). In a solving run each proposal passes preview admission (BND.61, BND.84, BND.85) and the
  program gets one fixed acknowledgement whatever the engine answered (BND.42, BND.43), while the shell keeps the full
  answer. In tool authoring and research a proposal becomes a capture and the engine builds nothing (BND.60). The flow
  rule stays off (BND.41): `read_state` feeds records only, and `LG3:capability.flow_refused` is never raised
  (CT-BND-24).
- Tests (`c600-run/tests/boundary.rs`): the saved-word half of CT-LG3-07, and parts of CT-FP-02, CT-BND-01 to
  CT-BND-04, CT-BND-07, CT-BND-11 to CT-BND-14, CT-BND-22 and CT-BND-26 to CT-BND-30, each naming its part.
- Drafts, which freeze with the IR or stay open past H-08's freeze: the acknowledgement is `{base, id}`, with the
  proposal number `<run>/<n>` as `id`; records are per run; the adapter serves a literal read of a name it stores as a
  workspace record and passes any other read to the engine, and a read with an argument always reaches the engine
  and counts as state-derived data for the block's `may_read_state` (LG3.13); the engine's preview call does not take
  the block yet (KN5.9); the store is in memory and returns a stored value as written, and M6's session store decodes
  by the reader's type.
- Picks (FP.38, BND.54; LP2's pull request 359): `Store::keep_pick` keeps a `c600-pick` record by its content digest,
  and admission resolves a block's `pick` after the contract and checker pins and before the author checks. This
  crate cannot depend on `c600-session`, so it reads a record with a minimal reader of its own
  (`provenance::field::pick_*`); `c600-session`'s `persist::pick` checks a record's payload when the store writes it.
  A pick whose record is not kept or does not read, or whose `chosen` differs from the record's, gives
  `PS1:capability.provenance_incompatible` (CT-BND-29's third case among them). A pick from a list that is no fixed
  entry is refused with `LG3:capability.not_granted` while BND.55 is open (LP2's draft, which fails closed), and a
  fixed entry's admission follows the block's parts (BND.85). Tests: `c600-run/tests/conformance_bnd.rs` also runs
  CT-BND-08, CT-BND-26, CT-BND-28 to CT-BND-30 and CT-KN5-13 with recorded picks, `c600-session/tests/persist_picks.rs`
  runs FP.38's vectors, and `c600-command/tests/kn5_admission.rs` admits a fixed entry's pick and refuses another.
- The command layer is LP3's `c600-command` (below); C5's bridge in `c600-tools` is the runtime's host onto it for one
  run, used through `Boundary`.
- Evidence kind: source and fixture.

## Status of M4: the 600-cell-Full kernel

- `Kernel::load` reads `assets/` directly. It checks that each listed name is a plain file name in `assets/` and that
  the manifest's `model_id` is the SHA-256 of its file list as `build_assets.py` writes it (0.4 does not check this),
  checks the SHA-256 of every listed file, and reads the archives with the crate's own ZIP, DEFLATE, gzip, NPY and JSON
  readers, so the kernel needs the standard library and `c600-digest` only. The identity is the manifest's `model_id`
  with the SHA-256 of the manifest's bytes.
- The mechanics follow 0.4's `core.py`, with the net-effect star cache under 0.4's guard and frame checks
  (ADR-L12.7, RTE.18). The integrator's decisions (a) to (e) are in the crate documentation: Net protection at label
  level; a recipe validated whole, under 0.4's limits and texts, before any effect; the trace's `model` block from the
  loaded identity; `progress_sha256` as the LX-6 port computed it; and the frozen case files as the only inputs, so no
  step needs NumPy.
- `tests/test_kernel_oracle.py` builds the adapter in release and requires `tools/oracle/compare.py` to report O01 to
  O07 equal with no omitted fields (3,201 steps), a tampered state hash to fail at its step, a case with another model
  identity to be refused without a trace, and the KN2.9 agreement of the star cache with per-primitive replay over
  210 stars. `tools/ci/run_headless.py` lists it.
- The same check runs the M4 differential leads. The new cases N01 to N04 (PHS.18; leads L5 to L7) replay equal over
  1,687 steps; `tools/oracle/make_new_cases.py` built them in the pinned engine environment, and
  `tests/fixtures/oracle-new` holds them with their own manifest. A recipe whose JSON 0.4 refuses step by step is one
  invalid-input step (L1), and so are 32 malformed inputs beyond O07 (L4). Fourteen traces from seven runs in other
  orders are byte-identical (L2). `core.py`, `session.py` and `make_cases.py` match their pinned digests (L3). The
  refusal and certificate texts are 0.4's. No float type or literal reaches a decision path in the kernel's or the
  digest crate's sources (CT-KN2-05).
- The adapter reads each step's recipe leniently (`json::parse_lenient`) and the rest of a case file strictly. A case
  with a repeated member name, NaN or an infinity is unreadable to both `compare.py` and the adapter.
  `Kernel::letters` refuses another model's recipe with `KN1:input.identity_mismatch` (KN1.4).
- `c600-kernel/tests/load.rs` tests the loader's refusals: a model ID that does not name the file list, a changed or
  missing listed file, a needed file that is not listed, names that are not plain file names in `assets/`, and
  malformed manifests. The checks inside the verified archives (a primitive that is not a permutation of its support,
  malformed offsets, an execution tree whose walk does not end) are not tested yet: such a test has to write a changed
  archive under a new model ID, and the crate only reads archives.
- Contract tests, each a part that names what it leaves to M5 or M6: CT-KN1-01, CT-KN1-04 and the labels half of
  CT-KN1-05; the 600-cell-Full halves of CT-KN2-01, CT-KN2-04 and CT-KN2-05; the labels half of CT-PS1-04; CT-PS1-12
  without Strict protection.
- Draft choices: a failed star certificate or asset check has no code in chapter 13's catalogue yet, so the kernel
  reports a `CertificateError` or `LoadError`; an orbit outside 0 to 34 in a protected set is refused with
  `KN2:input.named_object_invalid`; with one model in `assets/`, the other-identity control changes the case instead
  of loading a second model.
- Not built: `open_snapshot` by `RevisionRef` (with the session store, M6), the reachability claim of `Derived`,
  budgets (KN7), an inverse call (O05's inverse recipes come from 0.4's `core.invrecipe` in the case files) and the
  jumbling half (M5). The `positions` row, Strict protection and streaming words were built after M4 (below).
- Open: M6's `state_from_labels` builds a snapshot at the same level from any permutation of the slots, so CT-KN1-04
  lists it as no lift; reachability stays the caller's evidence. The model check on protection was open here and is
  built after M4 (below).
- Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the kernel with the
  acceptance check. Its two major findings were fixed before the pull request, and a scoped re-check found no blocking
  finding: R1, the model ID was not tied to the file list; R2, the loader's refusals had no tests (the checks inside the
  archives stay untested, above). Deferred: R3, `Refused` is not yet KN2.11's
  `{first_refused_prefix: Nat, outcome, certificates}`, because Net protection judges the whole net map and has no
  first refused prefix, so what a Net refusal reports needs a decision; R4, a failed certificate has no catalogue code
  (draft choices, above); the second half of R5, the ZIP reader's duplicate-name check is quadratic in the member
  count, on digest-verified bytes only. Fixed: the first half of R5, a drive or stream colon in a listed name. No
  action: R6, the KN2.9 agreement and the cached path share the inverse-move handling, which O01 to O03 cover.
- The LP2 tests' two kernel fixes, the lenient recipe region and `letters`' refusal, each had a fresh Sonnet read; the
  first read came after its commit on the agent's branch and before integration, the second before its commit. Neither
  found a blocker or major finding. Its two suggestions, that a repeated name or a non-finite number in a recipe be one
  invalid step, were refuted: `compare.py` refuses such a case whole, as the adapter does, and
  `check_unreadable_recipe_json` pins that agreement.
- Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg was not run. The timings the check
  prints are informational, not performance evidence.
- LP2 kernel rows after M4, which LP3 waits for (the crate documentation lists the draft choices):
  - KN1.4 for protected sets: a nonempty `ProtectedSet` is bound to its model, and `checked_protection`,
    `strict_protection`, `derive`, `derive_strict` and `replay` refuse another model's set with
    `KN1:input.identity_mismatch`. `protection` keeps its signature and stays unchecked. The empty set names no model.
  - Strict protection (KN2.4, KN2.7): `ProtectionMode::Strict`, `strict_protection` and `derive_strict` judge every
    executed prefix of the literal expansion; `derive` stays Net. On the shipped model every generator moves a sticker
    of each of the 35 orbits, so Strict refuses any word at prefix 1 once one orbit is protected; the later-prefix
    tests run on a copy of the model whose generator 1200 leaves orbit 0 alone.
  - KN2.11's refusal shape (deferred R3): `Refused` has `first_refused_prefix`, the outcome `refusal` and
    `certificates`, and `Protection` has `first_refused_prefix`. Draft: a Net refusal reports no prefix, since Net
    judges only the end of the complete operation (LB2.3); its certificates come from the net map.
  - The `positions` row: `positions(state, pieces)` gives each named piece's home slots, the slots that hold its
    stickers now and the position that holds them all.
  - Streaming words (KN2.10): `chunks(recipe, size)` streams a recipe's literal letters with their positions, one
    step's expansion at a time; `replay(base, set)` takes letters in chunks of any size, judges Strict prefix by
    prefix and gives no verdict before `finish`; `replay_under_ceiling` counts the 3,000,000 ceiling over all chunks.
  - Label-level protection of named pieces (for LP3's C8 and LB3.4): `protected_pieces` names any of the 177,120
    pieces, fixed cell centres included, as `positions` names them, and a nonempty set is bound to its model as a
    `ProtectedSet` is. `piece_protection` (Net, on an effect's net map) and `strict_piece_protection` (Strict, every
    executed prefix of the literal expansion) give one row per piece: the slots of its stickers in the base, which are
    the base's `positions` slots, and after the judged map. A centre's row is computed like any other, and since no
    letter moves a centre's sticker it comes out clear. The calls judge only: `derive` still takes a `ProtectedSet`,
    and a refusal for pieces stays the caller's, with no new code. `c600-kernel/tests/pieces.rs` checks the rows
    against `positions` before and after `derive`, the centre of cell 0 under H_0 (CT-LB3-02's label half from the
    kernel), all 600 centres under every generator, Strict against a scan that derives every prefix, and the set's
    refusals; `tests/load.rs` checks KN1.4 for both piece calls on a second model. The critical-path read is recorded
    with the slot action (M5's section).
  - The structure rows (draft, for LP3's C6, step 1 item 3): `compare_cells(first, second)` gives the fewest face steps
    between two cells (`structure.compare`), and `cell_layers(cell)` and `layer_counts(cell, layer)` the positions and
    slots of each breadth-first layer around a cell (`filter.preview`'s counts for a layer predicate), as 0.4's
    `cell_distances` and `layer_mask` give them: a position with a home slot in a cell of the layer counts whole, so
    layers overlap. `layer_members(cell, layer)` names what those counts count: the layer's positions and their home
    slots, each list strictly increasing (`structure.layer`'s draft read, pull request 375). Two cells share a face
    when they share three vertices, read from the 120 vertex pieces; the load checks the graph. The rows read no state.
    A bad cell or layer is `KN2:input.named_object_invalid` with the text of 0.4's `cell_mask` or `layer_mask`.
    `CommandLayer::compare_cells`, `cell_layers`, `layer_counts` and `layer_members` read them at the head and, like
    `CommandLayer::positions`, refuse with Jumble on (LG2.8). `c600-kernel/tests/structure.rs` checks every step count
    and every layer's counts against digests taken from 0.4's `core.py` on the same assets, and that a layer's members
    are as many as its counts, its slots their home slots, and a cell's layers together every position.
  - Contract tests: CT-PS1-12 whole; the kernel halves (parts) of CT-LB2-02, CT-LB2-03, CT-LB2-04, CT-LG2-12 and
    CT-LG2-14 (each whole by union with the language halves, and CT-LG2-12 also with `c600-jumble-tool`'s `lb3.rs`);
    CT-KN2-07 as a part: a 100,001-letter Full word replays in chunks equal to its whole derivation. Not run:
    the recorded words of CT-KN2-07, which are jumbling words, and any Full stream beyond 3,000,000 letters in the test
    profile.
- KN3 (LP2): `c600_kernel::exact` decides the sign of a + b√5 exactly for 128-bit a and b (`sign5`, a² against 5b²
  in 256 bits) and evaluates sums of products of integers, √5 and 5 with checked arithmetic that reports overflow
  instead of deciding. `c600_kernel::request` holds A6's bytes (state contract, in force since 15 November 2026): the
  wire limits, the four KN2 codes of KN3.7 and KN3.8, the FP.5 binding with its canonical text, the search limit, the
  request digest, builders for C, H and M lines, and the producer's token writer from a binary64 bit pattern (KN3.9).
- Contract tests, in part: CT-KN3-01 (Lucas and Fibonacci pairs at |a² − 5b²| = 4 up to the 128-bit limit, the range
  limits, exact zeros of contact type), CT-KN3-04 (the binding and the search limit) and CT-KN3-06 (bit patterns to
  exact tokens, both zeros to `0:0`, non-finite patterns refused). The rest of these rows runs in M5's
  `c600-jumble/tests/kn3.rs`.
- Draft choices: the bytes, limits and fixed-width signs live here with the standard library only, and a request's
  meaning, over unbounded integers, in `c600_jumble::rotation`; the shell passes a bit pattern, so no float type enters
  the kernel; a non-finite producer input and a malformed binding refuse with `KN2:input.rotation_request_malformed`;
  N or D above `u128` refuses with `KN2:resource_limit.rotation_search_bound`.
- KN3.5 (LP2): `c600-a2-filter` holds state contract A2's filter. It is a crate of its own because CT-KN2-05's scans
  keep float types and literals out of `c600-kernel` and `c600-jumble`; INV-4 and ADR-L03.1 allow the filter its
  float. It evaluates a sum of products of integers, √5 and 5 (the form of `c600_kernel::exact::SumOfProducts`) in
  binary64 and accepts the sign only when |v̂| > E = fl(Ŝ·4(K + 1)u), at least 3.99 times the proven forward error
  bound. Near zero, on a non-finite value and past 2^16 terms or 16 factors in a term, `c600_kernel::exact` decides;
  when its 128-bit path overflows, the decision says so. `Decision` records the version
  `a2-binary64-sum-of-products-draft-1`, the enclosure, and the accepted sign or the exact fallback result.
- Contract tests, in part: CT-KN3-03 (`c600-a2-filter/tests/adversarial.rs`, 12,400 cases). Every decision is
  checked against `c600_kernel::exact` and a wide-integer reference, and every finite enclosure is checked exactly.
  The cases cover Lucas and Fibonacci pairs, exact contacts of h_A's form and the crossings next to them, conversions
  that round, the 128-bit limits, cancellation-heavy sums, powers of √5, non-finite values and sums past the limits.
  Not run: the model's own vertex sets and poses and the E0 controls, because neither kernel crate calls the filter
  yet.
- Draft choices: the crate's own `Sum`, because the kernel's `SumOfProducts` keeps its terms private; the limits; the
  fixed √5 bit pattern; a margin of almost 4 with exact conversions counted as roundings; a fallback on a tie and on
  a non-finite evaluation, which records no value; a new `FILTER_VERSION` for any change to the bound, the constant,
  the evaluation order or the limits. The bound needs each operation rounded once to binary64, so the crate refuses
  to compile for 32-bit x86 without SSE2.
- Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the crate against
  KN3.5, CT-KN3-03 and state contract A2 and found no blocker or major. Its minor point, that halving E would pass
  the tests, is the margin the proof allows; the author's deliberate faults (a zero bound, the bound over 2^20, no
  non-finite check, the summation count left out of K, √5 off by one ulp) each fail them.
- Not built: the shell's gesture producer, the cost of an exact operation by operand size (KN3.6),
  and a call of the filter from either kernel crate; an accessor for `SumOfProducts`'s terms would let the filter
  take the kernel's sum directly.

## Status of M5: the jumbling half

- `c600-jumble` is a candidate engine for the rigid jumbling puzzle under the state contract with amendments A1 to
  A6 (`state-contract 2026-11-15 A1-A6`, the pin JI.3 names; owner decision, 16 November 2026). It is built from
  `assets/` alone and reads nothing from the research reference J1 (PHS.18); its import
  scan under `differential/independence.py` finds no harness or J1 module in the Python client
  (`research/jumbling/differential/candidate/rust_jumble.py`). The integrator's selections 1 to 6 (exact numbers,
  record families, free mode, Net protection at pose level, batches, the classification budget) are in the crate
  documentation.
- The jumbling model is read through `c600_kernel::verify_assets`, the kernel's asset check made public for it, which
  `Kernel::load` also calls: plain file names in `assets/`, the puzzle name, the model ID as the SHA-256 of the file
  list, and the SHA-256 of every listed file. The identity block's model ID therefore names the files that were read.
  `c600-jumble/tests/assets.rs` tests the refusals, and `c600-kernel/tests/load.rs` the check's result.
- JI.52: the harness bindings reach J1 only through its public API, `research/jumbling/sim/api.py`.
- `tests/test_jumbling_rust_candidate.py` builds the adapter in release and compares it with J1 through
  `harness.compare`, step by step (status, moved count, moved set, pose digest and lattice digest): the first 60
  records of each W-J fixture with surveys at steps 0 and 30 and the straddler sets of named caps at step 30, every
  candidate certificate checked by J1; the first 600 letters of H6030; refusals (a blocked twist, a twist outside the
  menu, a malformed record); and a planted difference, which fails at its step. With `JUMBLING_FULL_REPLAY=1` every
  record of the three fixtures and all 3,156 letters of H6030 run. `tools/ci/run_headless.py` lists it.
- Contract tests: CT-KN1-02, CT-KN2-03 (`c600-jumble/tests/crossing.rs`) and CT-KN3-02 whole; the jumbling halves
  of CT-KN2-01, CT-KN2-04, CT-KN1-05 (poses) and CT-KN2-05 (no `f32` or `f64` in the kernel crates), which run whole
  with M4's halves; CT-JI-03. CT-KN2-02 whole: heights and domain IDs against `trees.py` and the library's
  histograms on H6030's compared states, the recorded fixture prefixes and u1, the free-mode Cayley rotation, and
  (`ItemEightStateTests`) the rest of ENG-0J item 8's states: the word-check cases b20, b06-14, b14 and b03 at their
  start and end, the I_a@40 stop, the deep scramble and the six height-4 states, each with a planted height and a
  planted domain ID that fail. Where J1's own replay takes minutes or hours, J1's state is rebuilt from the
  candidate's pose table and must reach the research run's recorded digest. b14, b03 and the stop always run; b20,
  b06-14, the deep scramble and the height-4 states run with `JUMBLING_FULL_REPLAY=1`, since b06-14 alone takes
  about 34 minutes on a development machine.
- Not built, for M8: CT-KN3-03's model inputs (the filter itself is in `c600-a2-filter`, M4's section), the jumbling
  words of CT-KN2-07 and CT-KN3-01's small-margin set; the observation nu (M8 builds rim_summary, covers, repair_list
  and release_check, below); and the holding caps of seam cells and tree positions past the first step. Built since M5:
  KN3's exact rotation requests (below; CT-KN3-05 whole, CT-KN3-01, CT-KN3-04 and CT-KN3-06 in part), the kernel rows
  for 600-cell-Full (M4's section), and M8's levels, end tests and derived values, Strict protection included (below).
- Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the change with the
  acceptance check. Its three major findings were fixed before the pull request, and a scoped re-check found no
  blocking finding: R1, a word whose later letter was malformed, or met an engine error, left its earlier letters
  applied (the adapter now reads every letter before it applies any, and restores the state on every error); R2, a
  record step before `start` stopped the adapter with a panic (it is now an input error); R3, the model ID was taken
  from the manifest unchecked (the model now loads through `verify_assets`, above). Fixed: R4, a record of a family the
  candidate does not read was refused as `invalid` when its matrix was malformed (the family and a float request are
  now checked first, so it is an adapter error under selection 2); R5, `seam_facts` on a cap whose classification ran
  out of budget claimed a complete answer (it now claims nothing, JI.14). No action: R6, a refused word still reports
  the end state it reached, with `admissible` false and the failing letter's index, as J1's `word_check` reference does
  and the comparison checks; R7, the INV-4 half of CT-KN2-05 is not scanned, and the test's docstring says why; R8,
  CT-KN3-02's large entries come from rotations the test chooses, which the engine makes exact and `load` reads back
  with the rotation and the digest checked exactly, and J1's digest agrees on the first.
- Attack tests from the Fable solver's leads of 16 November 2026: `tests/test_jumbling_rust_attacks.py` (through the
  adapter), `c600-jumble/tests/refusals.rs` and `c600-jumble/tests/domain.rs`. CT-JI-03 in part (cancellation yields
  only covered prefixes; the fixed core cell is listed in the identity domain), CT-KN2-04 in part (a blocked word
  check and refused named objects change nothing) and CT-KN2-02 in part (the free-mode pose's domain ID names the
  coset g K+: g k keeps it for every k in K+, k g only when g^-1 k g lies in K+); the other tests check JI.3, JI.5,
  JI.6, JI.9 to JI.12, JI.39, JI.45, JI.52, JI.53, KN1.4 and KN2.14.
- Fixed after those tests. R6 is reversed: a word check whose prefix is refused makes no end-state assertion (JI.11);
  its 22 end-state fields are null with `end_state_undefined` "refused_prefix", the start facts, the failing index
  and the processed range stay, and with a named unit the step flag is false (LB3.8 checks every prefix). A cap
  outside 0..599 in `word_check`, `prefix_observations` or `seam_facts`, and a negative piece ID in a named unit,
  were adapter input errors; they are now refused with `KN2:input.named_object_invalid`, and every such refusal
  names the argument, the supplied IDs and the failed condition (JI.45). A request or a state record may name its
  `dimension`; any value other than 4 is refused with `KN1:input.unsupported_dimension` before any other field is
  read (JI.3, JI.45).
- Drafts: a request or record without a dimension means 4; a refused prefix's processed range is [0, index + 1),
  the refused letter examined and the suffix unchecked.
- J1 faults, reported and not fixed: `references/word_check.py` describes a refused prefix's end state and gives a
  boolean step flag without a named unit (JI.11), and accepts a lattice piece or a mixed-pose set as the unit (JI.9);
  `sim/api.py`'s `fixture_state` silently replays another prefix for a t outside 0 to the record count, and its
  `pose_entries` hands out J1's own exact numbers, which take attribute assignment, so the "read-only copy" can change
  the state's pose table (JI.52). The attack tests keep them as expected failures.
- Not run, because the candidate does not build the calls: lead L9 (`label_checkpoint` on a raw table with a turned
  centre) and L16's `KN2:stale.revision_gone` (the candidate has no snapshot handles). L15's `repair_list` and
  `covers` rows (J1's pinned pair order, no rank, a cap outside 0..599) and L2's `release_check` agreement run in
  M8's comparison with J1 (below).
- Evidence kind: source and fixture. The comparison ran on Windows; its headless Linux leg runs in CI (the Linux Rust
  workflow's J1 job). The timings the check prints are informational, not performance evidence.
- KN3.4 and KN3.7 to KN3.9 under A6 (`c600_jumble::rotation`): `Request::decode` reads C, H and M lines with unbounded
  integers, refuses malformed, noncanonical and oversize bytes, and compares the re-encoded bytes. `resolve` builds the
  exact rotation in A1's frame at the binding's cap and chooses: free mode over A1's grid with A1's tie order, half-turn
  candidates for H and for C at ω·ω ≥ 13130, and the squared distance exact in Q; menu mode by exact membership or a
  drag scored by the squared Frobenius distance in Q(√5), the smallest index winning ties. The record holds the
  binding, the request digest, the choice (with the complete tie list for a drag) and the exact matrix; `replay` reads
  it back through the exact rotation check and runs no chooser. On 600-cell-Full, `retained_word` turns a retained
  item into its A4 word and refuses a jumble item with `KN5:input.menu_word_needs_jumble_on` (JI.35). The A6 codes join
  `codes`.
- Contract tests (`c600-jumble/tests/kn3.rs` and `rotation::tests`): CT-KN3-05 whole (LX-9's vectors: the quarter
  turn's drag picks 15 of the ties 15, 29, 40 and 43; the reversed menu's order digest is 1a87a420…, old bindings
  refuse and its drag picks 16, the old 43; and H requests: the frame axes' half-turns are the direct members 11, 1
  and 10, H(1,1,0), H(0,1,-1) and H(1,2,3) refuse with `KN2:input.twist_off_menu`, and the H(1,1,0) drag picks 23 of
  the ties 23, 24, 41 and 54 at (9 − 3√5)/2); in part CT-KN3-04 (not run: a search for N ≥ 1 near the limit),
  CT-KN3-06 (not run: the shell's gesture producer), CT-DET-16 (identical requests give identical choices and records
  against pinned digests; not run: the shell's gesture producer and another platform) and CT-LG2-16's kernel half
  (not run: the language half).
- Draft choices (`rotation`'s documentation): half-turn candidates are the canonical integer axes with every
  |component| ≤ N, at denominator 1; free mode compares the whole grid, (2N+1)³·D Cayley points and the half-turns,
  with no budget or cancellation and no pruning except at N = 0, where the grid is the identity alone and one
  comparison decides; the order of refusals; 600-cell-Full resolves against the pinned menu and refuses a free binding
  with `KN2:input.menu_binding_mismatch`.
- Unchanged: selection 2 (a journal's float request is an adapter error). `CONTRACT_REVISION` now names
  `state-contract 2026-11-15 A1-A6` in the identity block and in the repair list's parameters (owner decision,
  16 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-16-lp2-questions.md`). J1 still stamps
  `state-contract 2026-10-09 A1-A4`, a known J1 defect that stays in J1; `tests/test_jumbling_rust_m8_observations.py`
  maps J1's stamp to the engine's in its comparison and nowhere else, and KN1.4 refuses J1's stamp at the candidate
  (`tests/test_jumbling_rust_attacks.py`, `StampTests`). KN3.3's exact double description is `dd`; the polytope
  operations over it came later (`polytope`, the KN3.3 bullet below).
- M8 observations (LP2 plan, section 11; JI.6, JI.7 and JI.12): `c600-jumble/src/observe/cuts.rs` builds
  `rim_summary`, `release_check`, `covers` and `repair_list`, exact and read-only. The adapter serves them and
  `rust_jumble.py` calls them. `tests/test_jumbling_rust_m8_observations.py` compares each with J1 on the states J1's
  own tests use, undefined fields and refusals included: the rim summary on the I_a@40 stop and the I_a prefixes 10,
  20 and 60; on census seed 7101, the release check before steps 44, 70 and 97, the cover table before steps 17, 30
  and 33 (true answers in both directions included), and the repair list before step 97 at cap 458, whose 472
  outcomes pass J1's `compare_recorded`, with J1's live cut-off, cancellation and errors.
  `c600-jumble/tests/observations.rs` tests the refusals and the agreement with M5's seam facts and word check.
  - A named cap outside 0..599 is refused with `KN2:input.named_object_invalid` (JI.45); a letter's grip outside it
    stays an input error, as in the word check.
  - In `covers` and `repair_list`, a classification or pair budget that runs out cuts the job off with
    `KN7:budget.kernel_cost` and no claim (KN7.4). In `rim_summary` it leaves the caps it could not classify
    uncertain, with no rim and no grip N.
  - The four drafts, as the owner decided them on 16 November 2026 (PHS.52): the pair budget `max_pairs`, which JI.6
    and JI.12 do not define, takes `KN7:budget.kernel_cost` once JI declares the budget (KN7.10 asks for that budget's
    code); a cancelled repair list keeps its covered outcomes and needs no code (KN7.3's `cancelled`;
    `KN7:cancelled.by_user` exists if one is wanted); a blocked or closed named cap, and a repair list in free mode,
    get new rows in chapter 13's amendment rather than a wider `KN2:input.named_object_invalid` or
    `KN2:input.menu_binding_mismatch` (ERR.5), and stay errors with no code until then. Chapter 13's amendment at
    LP3's command-table freeze registers the two rows, `KN2:input.named_cap_blocked_or_closed` and
    `KN2:input.menu_order_unpinned` (its section 9); the errors take the codes when the crate names them.
  - Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model found no blocker or
    major finding. Its minor points are the drafts above, which it found defensible, and engine faults that return a
    plain error without a code, as the crate's other engine faults do; no change.
  - Not built with them: `nu` and `correction_set_check` (JI.13). `nu` needed the label level of A5's crossings, and
    the check needed `nu` and the 36-row correction catalogue; both came later (the ν and correction catalogue bullets
    below).
  - CT-KN2-01, CT-KN2-04 and CT-BND-29 in part; no state changed in the index. No J1 fault was found. Evidence kind:
    source and fixture; the comparison ran on Windows, its headless Linux leg runs in CI's J1 job, and its timings are
    informational.
- M8 levels (LP2 plan, section 11; chapter 01, KN1.7 to KN1.11, KN1.14, KN1.17 and KN2.11 to KN2.14; chapter 08,
  ONE.21 to ONE.28; state contract A3 to A5): `c600-jumble/src/levels.rs` builds KN1.7's level crossings, each map
  one way from poses down, and KN1.9's projections, total or partial with what is missing (the pieces off the lattice
  first); A5's end tests, the sticker test, the retained checkpoint and the label checkpoint under a reachability
  claim; shapes and gauges with their classes S, R and all (KN1.14); and derived values: `derive` from a witnessed
  snapshot or an earlier derived value under Net or Strict protection at pose level, KN2.14's `import` of a raw pose
  table with an open or promised claim, and ONE.25's `compare`. The adapter serves them, `rust_jumble.py` calls them,
  and `tests/test_jumbling_rust_m8_levels.py` compares them with J1 on J1's fixtures (`tools/ci/run_headless.py`
  lists it). Every word is the caller's or a fixture's; nothing chooses a twist.
  - A verdict carries its level (KN1.11, JI.4): the sticker test and the label checkpoint answer at the labels level,
    the retained checkpoint at the pose level, and the adapter's answer names it.
  - A witnessed claim comes only from a replay from solved or from a derivation, and it speaks only of the state it
    reached (A4, A5): for another state the label checkpoint answers unknown, and its retained witness shows no ν
    there.
  - A raw table with two pieces in one chamber reads (KN2.14 validates form and identity only) but is no label
    checkpoint: `pieces_collide`, with the collision count, whatever the claim.
  - The label checkpoint's certificate carries the engine's stamp, `state-contract 2026-11-15 A1-A6` (owner decision,
    16 November 2026). The ν reference's label certificate stamps A1 to A5, and the levels comparison maps that stamp
    to the engine's, as the observations comparison maps J1's.
  - Drafts, in the crate documentation: the label checkpoint answers no for the facts that do not depend on
    reachability (off the lattice, a displaced centre, a collision, a known nonzero ν) whatever the claim, as J1 does,
    though ONE.28 can also be read to leave every claim but a witnessed one at unknown; by-theorem and refuted claims
    answer unknown, since this edition checks no theorem evidence; an unknown level name, an invalid gauge, an import
    with a witnessed, by-theorem or refuted claim, a malformed table, a journal that does not witness its base, an
    unknown derived name and a non-twist letter are adapter input errors without a code, as chapter 01, the interfaces
    and chapter 13 name none; a protected piece outside the model is `KN2:input.named_object_invalid` in `derive`; an
    invalid letter refuses the word with `KN2:input.word_blocked_prefix`; the default mode is Net, and with no budget
    an uncertain letter is an engine error; derived values are named by the caller and replace an earlier value of the
    same name.
  - Critical-path reads (`AGENTS.md`, Codex minimum): fresh Claude subagents on a smaller model read the three rows. Row
    3's read found no blocker or major finding (one minor fixed, one rejected, two nits deferred: a derived claim nests
    its base's evidence one level per derivation, and the lineage root is a digest until M6). The read of rows 1 and 2
    found three major findings, fixed before the pull request: a verdict named no level, a witnessed claim was not tied
    to its state, and a collided raw table gave an engine error instead of a verdict. A scoped re-check by a fresh
    subagent found the three fixed and no new blocker or major finding; one minor point is kept (a witnessed claim made
    for another state answers unknown even where the labels are the identity, as A4 and A5 allow), and two nits are
    deferred (an `Undefined` arm that cannot be reached, and the claim's new `at` field, which no J1 reference
    compares).
  - Contract tests: CT-KN1-03, CT-KN2-06 and CT-ONE-10 whole; CT-KN1-04 whole by the union of M4's 600-cell-Full half
    and this jumbling half; CT-KN1-06, CT-PS1-14, CT-ONE-09, CT-ONE-28 and CT-LG2-07's engine half in part, each
    marker naming what it leaves out. Not built with them: `nu` and `correction_set_check` (the ν and correction
    catalogue bullets below), and the session store's checkpoint and import records (M6). No J1 fault was found.
    Evidence kind: source and fixture; the comparison ran on Windows, and its headless Linux leg runs in CI's J1 job.
- The kernel pieces for LP3's C8 (LP2):
  - The slot action of K⁺: `levels::Tables::slot_image` is A3's k·s in one call with its frame check (M8's `image` and
    `slot_of` give its parts without the check), and `piece_slots` gives a piece's slots at a lattice pose in the order
    of its home slots; `labels` now runs on them with unchanged results.
  - The engine-named rotation plane: `twist::rotation_plane` (and `Twist::plane`) names a rotation's plane exactly
    for a swept preview (render/1, section 4). u is the first nonzero column of g − I; v is g u − cos θ·u for θ < π,
    and for a half-turn the first later column independent of u made orthogonal to u, which is the engine's naming of
    the half-turn's direction. u and v are unnormalized, and R(θ) built from them is g exactly. The identity, a
    rotation that turns two planes and a matrix that is not a rotation get none. The display's `f32` plane is LP3's to
    fill from it in `c600-display`.
  - `c600-jumble/tests/slots_planes.rs` tests the left action on 21 elements, the agreement with `labels` after a
    retained word, the refusals, and the planes of the 12 elements fixing pole 0, every retained generator, every
    seventh element of K⁺, H_0 and an off-lattice Cayley rotation, each rebuilt exactly.
  - Nothing more was built for LB3.4: its projection from poses to labels with the checkpoints, and a whole word's Net
    and Strict protection at pose level, are M8's levels (above: `project`, `retained_checkpoint`, `label_checkpoint`,
    `derive` with `WordProtection`); CT-LB3-06's peak, excursion, tameness and step flag are M5's `observe`; label-level
    protection of a piece is the kernel's (M4's section).
  - Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the slot action,
    the plane and the kernel's piece protection with the acceptance check and found no blocker or major finding. Its
    two minor points were fixed before the pull request: KN1.4 for the piece calls is now tested on a second model,
    and Strict is compared with a scan that derives every prefix. No action on its two nits: an effect names no model,
    as M4 decided, and the identity pose skips the frame check, as before.
  - Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg runs in CI.
- ν, the label invariant of A5, over F2 and F5 (LP2, for LP3's C8; chapter 01, KN2.4's `nu` row and KN3.2; state
  contract A5):
  - `c600_kernel::exact` gains `F2` and `F5`, KN3.2's prime fields of ν and the corrections: a residue with the field
    operations, a checked inverse and division, powers and `Display`. Unit tests check the field laws.
  - `c600-jumble/src/nu.rs` gives A5's 45 coordinates in their fixed order: b0 to b34, then the frame bits f0, f3, f6,
    f8, f11, f22, f25 and f32 in F2, then C5_17 and C5_28 in F5. It is a port of restoration stage 03's `nucoords.py`.
    The labelling and the frame groups are built once from `assets/` (the execution trees, the seed atlas and the
    census) and checked against the census; every piece but the 600 centres is labelled. The model keeps the three
    files' verified bytes from its asset check, so the labelling comes from the files that the model ID names, and a
    manifest that does not list them is refused (`c600-jumble/tests/assets.rs`).
  - `nu::value` gives ν of any state. It is undefined off the lattice (`off_lattice`) and for a raw table with two
    pieces in one chamber (`pieces_collide`).
  - `nu::nu` is the row on a snapshot or a derived value. On a state of the I_a menu whose claim is witnessed at that
    state, a nonzero coordinate that A5 fixes (b0, b1, b2, b3, b5, b6, f0, f3 and f6) is refused with
    `KN2:consistency.fixed_nu_nonzero` (chapter 13's form freeze). Open, promised, by-theorem, refuted and imported
    claims, a witnessed claim made for another state (as for the label checkpoint), other menus and free mode get the
    value with no check.
  - The I_a menu is recognised by its order-free identity. Its order digest is pinned with the other constants for a
    caller that binds the order: the C5 convention (r0, the least frame permutation of order 5 at the third buffer)
    and the certificate program's SHA-256.
  - The adapter serves `nu` and `nu_constants`, and `rust_jumble.py` calls them.
  - `c600-jumble/tests/nu.rs` tests:
    - the tables against the census;
    - ν = 0 at solved and after retained words, under I_a and in free mode;
    - ν undefined off the lattice;
    - a turned piece of orbit 0 sets f0 alone, and an open claim is not bound;
    - a collided table is undefined.
  - `tests/test_jumbling_rust_nu.py` tests EH-10 against the ν reference:
    - ν = 0 after retained words;
    - the research tables E3, I1 and I2 equal `nucoords.py`, TABLE-NU and the restore summary;
    - turned pieces of orbits 0, 6, 17 and 34 reach every frame class, checked through J1's K⁺;
    - corrections 0 and 2 from solved give their table rows;
    - the pinned constants and frame groups match `nucoords.py` and the census.
  - Not run: the refusal on a witnessed I_a state. Stage 04's theorems leave no such state to build, so unit tests
    cover it with a planted value: the consistency check, and the row's decision on the menu identity, the claim's
    kind and the state the claim was made at. The language rows that use ν (CT-LB1-01, CT-LB3-01's ν case, CT-LB3-04
    and render row J12) are LP3's. `correction_set_check` and its catalogue (EH-11) are the next bullet.
  - Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model compared the port
    with `nucoords.py` and found no blocker or major finding. Three minor points were fixed before the pull request:
    - the row's decision is now unit-tested;
    - a witnessed claim made for another state now gets the value instead of an engine error;
    - the retained words now run in free mode too.
    No action on its two nits: the group test is not tautological, since `order` counts the group's elements as read
    from the trees, and a label check that is redundant but harmless stays.
  - Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg runs in CI.
- The correction catalogue and the drafted correction-set check (LP2, for LP3's C8; chapter 01, KN2.4's
  `correction_set_check` row; JI.13; DT-J1 O6 (b) and O7 (a), owner, 22 October 2026):
  - `c600-jumble/data/corrections-I_a.json` holds the 36 corrections of the pinned research library
    (`research/jumbling/restoration/09-general-method/data/library-merged.json.gz`, decompressed SHA-256 8a13eb6c…),
    each with its id, its family, its literal word as (grip, item) letters of the pinned I_a menu (23,932 letters in
    all, 5 to 5,629 a word) and its 45-value ν row. `research/jumbling/differential/references/catalogue.py` writes
    the file from the library. The file is bound to the model ID, the I_a menu identity and order digest, and the
    contract revision. It is data of the jumbling engine outside `assets/`, so the model identity does not change.
  - `c600_jumble::corrections` reads the file once from the crate and checks the format, the binding, the library's
    path and SHA-256, the coordinate names, the 36 ids in order, each row in its fields with the fixed coordinates 0,
    and each letter's grip and item; a defect is an engine error.
    - `Catalogue::lookup` gives a correction by id; an unknown id is refused with `KN2:input.named_object_invalid`.
    - `Catalogue::twists` gives a correction's literal word as twists of a menu that is I_a by identity and order
      digest, since the letters number I_a's items in their recorded order (`KN2:input.menu_binding_mismatch`), on
      the catalogue's model (`KN1:input.identity_mismatch`).
    - `Catalogue::predict` adds the rows of a drafted set of (id, m) pairs, m any integer, in F2 and F5.
    - `correction_set_check` is KN2.4's row: ν of the base (`nu::nu`), then the prediction. The base must be under
      I_a by the menu's order-free identity, as ν's gate is, since the check uses rows and not letters. Free mode and
      other menus are refused with `KN2:input.menu_binding_mismatch`. Off the lattice the result is undefined, and
      ν's own refusal passes through.
    - Nothing chooses, ranks or combines corrections: every id is the caller's (DT-J1 O5 and O6 (b)).
  - The adapter serves `corrections`, `correction_lookup` (with `twists`, the word as twists of the started state's
    menu) and `correction_set_check`, and `rust_jumble.py` calls them.
  - `c600-jumble/tests/corrections.rs` and the module's unit tests test:
    - 24 planted defects of the file, each an engine error, and three in-field plantings that still read;
    - another model's catalogue and other menus refused;
    - EH-11 from solved for the corrections of at most 37 letters (ids 0 to 16), with the check predicting each row
      and, from the played state with the id taken back, 0;
    - the check on a derived value;
    - signed multiples in F2 and F5 (m = 10 and five plays of a C5 row give 0; i64::MIN acts as two plays and
      i64::MAX as seven);
    - the check undefined off the lattice.
  - `tests/test_jumbling_rust_corrections.py` tests against the research library:
    - the shipped bytes against `catalogue.py`, and the adapter's catalogue against the library;
    - each word under I_a against the library's records, grip and exact matrix;
    - EH-11: all 36 from solved and, from the lattice research input E3, the corrections of ids 0 to 20 and 27 (b
      bits, the frame bit f8 and C5_17); with `JUMBLING_FULL_REPLAY=1` all 36 from E3;
    - the check against `corrections.drafted_nu` on solved, witnessed, stored and derived bases and on E3 as an
      import with an open claim;
    - the refusals, off the lattice, and malformed requests as input errors.
  - Not run: I1 and I2 as starts (the references ran all three in J1), and a witnessed I_a state with a nonzero fixed
    coordinate, which stage 04's theorems exclude. The language rows that use the catalogue (CT-LB1-01, CT-LB3-04,
    CT-LB3-01's ν case and render row J12) are LP3's.
  - Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the change with
    JI.13, KN2.4's row and EH-11 and found no blocker or major finding. One minor point was fixed before the pull
    request: the i64::MIN case compared `predict` with the `add_times` it calls, and now i64::MIN and i64::MAX are
    checked as two and seven plays. No action on the other three:
    - the check refuses a free-mode or other-menu base before it reads ν, as its doc comment says, where `nu` alone
      answers undefined off the lattice; KN2.4's row sets no order, and the rows are I_a's;
    - the off-lattice tests use eight journal records, which reach the off-lattice states they assert;
    - `twists: true` before `start` is an input error, as for the other ops that need a started state.
  - Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg runs in CI.
- KN3.3's polytope operations and the exact core of LB4.1's map property checks (LP2, for LP3's C6 and CT-LB4-01;
  chapter 01, KN3.3; chapter 03, LB4.1's map property row and LB4.2):
  - `c600_jumble::polytope` builds on `dd`. A `Polytope` is the hull of exact points of any dimension in any ambient
    space: its vertices in the order they first appear, the equations of its affine hull, its facets within the hull
    with the vertices on each, and a basis of its directions. The facets are `dd`'s vertices of the polar about the
    points' centroid. `from_constraints` takes `dd`'s inequalities.
  - Region operations, exact over the whole region: `locate` (the relative interior, the facets a point is on, or the
    first equation or facet it breaks), `contains`, `line_meet` and `ray_meet` (a line's interval of parameters, a
    closed ray's first point), `intersect`, `edges`, `relative_interior_point`, and `positive_functional` (an h with
    h·x > 0 on the region, which exists when the origin is outside).
  - `ProjectiveMap` is x ↦ (M·x + t)/(c·x + d), defined where c·x + d > 0: `new`, `affine` and `central(h)`
    (x ↦ x/(h·x)), with `apply`, `defined_on` and `image`. `Metric` is a symmetric positive definite Gram matrix
    (`euclidean`, `gram`) with exact squared distances.
  - The four checks each decide their own property over the whole region, never on the vertices alone (LB4.2), and
    give `Check::Holds` or `Check::Fails` with exact points that verify the failure. None reports another's property
    (LB4.1).
    - `injective(map, region)`: the homogeneous map's kernel on the span of the centroid and the directions. The
      witness is the two ends of a chord whose points share one image.
    - `embedding(map, cells)`: injective on each cell, and f(σ) ∩ f(τ) = f(σ ∩ τ) for each pair. The witness is the
      two preimages of a vertex of f(σ) ∩ f(τ) outside f(σ ∩ τ).
    - `isometry(map, region, source, target)`: a map that is not affine on the region changes one half of a chord
      through the centroid; an affine one is checked on the chords along the directions and their pairwise sums
      (polarization).
    - `homomorphism(map, cells, codomain)`: each cell's image is a cell of the codomain, by vertex set. Faces count
      when they are listed as cells. The witness is the first cell that is not, with its image.
  - Radial and stereographic charts reduce to `central(h)` for injectivity and embedding, with one h positive on the
    cells, and a pole's domain test is `ray_meet` from the origin along the pole. The module documentation states the
    reduction, and a caller that uses it cites it. The checks take no budget: a language entry that wraps one keeps its
    own (LB4.1). No float type is used.
  - `c600-jumble/tests/polytope.rs` and the module's unit tests test:
    - the faces of the 4-simplex, the 16-cell, the tesseract and the 24-cell (vertices, facets, edges, incidences and
      edge lengths), repeated and inner points, and a triangle, a segment and a point in 4-space;
    - lines and rays on the unit square, and a cell of the 600-cell with edges 1/φ: with a stereographic pole on its
      axis, no vertex is on the pole's ray and the region test finds the centroid on it (LB4.2, CT-LB4-01);
    - intersections, among them 2·(16-cell) ∩ tesseract = 24-cell and two tetrahedra that no face of either
      separates;
    - each check holding and failing, every witness checked again from its points;
    - failures that no vertex test sees (LB4.2): a central projection that is not injective on a full-dimensional
      region although its vertices' images all differ, two disjoint segments whose images cross although their four
      vertex images differ, and x ↦ 2x/(1 + x) on [0, 1], which keeps the vertices' distance and changes the halves;
    - malformed regions, maps and metrics.
  - Not built: the language entries over these checks and their results. CT-LB4-01 is built on these checks in LP3's C6
    (`c600-display`'s chart checks). The checks take projective maps; another class of map needs its own exact check.
  - Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg runs in CI.
- J10's exposed faces (JI.41, JI.45): `c600_jumble::faces` derives the cut 3-faces of the 36 K⁺-orbit
  representatives' regions exactly in Q(√5) from `assets/` alone and transports them to all 177,120 pieces by
  K⁺; no mesh asset is read or written, and the model identity, the manifest and render/1 are unchanged. A face is
  (piece, home pole, side) on the cell of chambers J08's seam rows name. `faces::certify` checks a table
  independently of the builder: the faces lie on their hyperplanes with affine rank 3, their 2-face cycles are
  convex and oriented by exact determinant signs (V − E + F = 2, 2V − 4 fan triangles), the hosts are the model's
  stickers and the boundary closes, every cut cell pairs up, and the counts are EN2's. The admitted size profile
  (`faces::PROFILE`) is measured headless in `docs/progress/1.0/evidence/j10-exposed-face-size-profile.json`.
  `faces::exposed_faces` answers with the certified geometry's digest and the profile, bound to the state; the
  faces are home geometry and the state changes nothing. Without certified geometry or the admitted profile it
  keeps LP2's refusal (pull request 336), `KN2:input.exposed_geometry_unavailable` with JI.45's context naming
  what is missing (`c600-jumble/tests/faces.rs`, `tests/refusals.rs`).

## Status of M6: the session store

- `c600-session` keeps one session in one journal on a `Disk`: `FsDisk` over a directory, or `MemDisk` in memory for
  the tests only, behind the crate feature `mem-disk`. `MemDisk` fails a chosen write (an error, a partial write, or
  a write that persisted but reported failure), holds a write so a test can act while it is in flight, tears unsynced
  appends and loses unsynced data in a crash, and counts reads and writes.
- The session lock (KN4.1: only the engine opens the session files; Fable lead L9). A store takes it before it
  reads the journal, so one process opens a session at a time. `FsDisk` holds an exclusive lock on the empty file
  `lock` in its directory (std's `File::try_lock`: `LockFileEx` on Windows, `flock` on Unix) until the disk is
  dropped. The operating system releases it when the process ends, a hard exit included, as it did for 0.4's
  `engine.lock` (1.0-A3's V03.4: a lock conflict is a typed error, not a crash). A second opener of a held session,
  in this process or another, is refused with `KN4:busy.session_locked` before it reads or writes the journal or a
  state object. That covers `Session::open` and `create` and `JournalStore::open` and `create`. Every disk takes
  the lock (finding E2 of the KN5 boundary attack, the owner's reading of 18 November 2026,
  `docs/wiki/decisions/owner-decisions-2026-11-18.md`): one clone of a `MemDisk` holds it until that clone is dropped
  or the disk crashes, so a second store on a held memory disk is refused the same way, and a clone opened after the
  holder is gone stands for a restart in the crash tests. A lock on a file holds only while the file keeps its name
  (E1). A journal without a `lock` file refuses the opener, and a holder puts back a removed `lock` when it is
  dropped. On Windows `FsDisk` opens `lock` through the platform layer `c600-platform` with a handle that denies
  delete sharing, so nothing deletes or renames it, or removes the session directory, while it is held; outside
  Windows a `lock` removed and created again while the session is held lets a second opener in. A hard exit leaves
  `lock` in place. When something outside the store removed it, `FsDisk::open_anyway`, the engine's explicit "open
  anyway" (no UI), puts it back and opens; it refuses while this machine's lock is held, and it does not see a lock
  that another machine or a sync service holds on a shared folder. This is not the lock of
  KN4.2's R-15, which is the in-process lock that 0.4 held while it wrote a response (B-005); M6 calls that one the
  core lock. Drafts: the file's name and the code, which chapter 13 did not register until LP3's command-table freeze
  registered it (its section 9; PHS.52).
- The journal: one durable action is one frame (its length, the length's complement, the payload and its SHA-256),
  appended and synced before the reply (R-06). When the append or its sync fails, the store cuts the journal back and
  syncs the cut; only a cut that succeeds proves the rollback, answered `failed` with `KN4:resource_limit.disk_full`.
  Otherwise the receipt is `unknown_outcome`, and the session refuses every row with
  `KN5:unknown_outcome.read_only_recovery` until it is reopened (KN5.23). Opening syncs what it read before it
  answers from it, cuts a torn final frame and syncs the cut, and opens any other damage read-only at the last whole
  frame (ONE.70); when the cut or its sync fails, the open fails and the next one cuts again. A torn final frame is the
  start of one frame followed by nothing or by zero bytes, as a file system that extends a file before its data
  arrives leaves it; the complement tells a damaged length from a torn append. A checkpoint's state is an object named
  by its digest, written and synced before the frame that cites it unless an intact copy is there, and read back
  through the kernel's new `state_from_labels`; a checkpoint whose object is lost or damaged opens by replaying the
  journal to it. `Session::create` refuses a disk that holds a whole or a damaged frame before it writes anything,
  cuts the torn remnant of a header, and answers a header whose fate is unknown with
  `KN5:unknown_outcome.commit_fate`.
- Revisions carry KN0.2's counters and the engine epoch. Snapshots open at the current revision, at an earlier
  revision, or at a checkpoint (LI.11, ONE.76); a revision stored in an earlier engine epoch opens again under the new
  one, because the reopen rebuilt the same counters (ONE.17). A commit after an undo starts a new line, one line is
  active, and a line switch is a commit row (ONE.73 to ONE.75). The status reads no storage (R-13).
- The commit path runs chapter 08 section 7 from step 3 in its order. A shell stub does steps 1 and 2: it mints
  single-use tokens bound to the session, epoch, command, row and its arguments, the identity and, for an execute,
  the preview on screen, under a per-launch secret (HMAC-SHA256 over `c600-digest`; KN5.14, KN5.15). A token also
  binds the `state` and `protection` revisions the action was resolved against: an execute's are its preview's, and
  undo, redo and a switch must find theirs unchanged (LI.6). Undo and redo take an `ActionHint` (LI.9, LI.26). Outcome
  lookup answers every durable command across a reopen, and an identical retry gets its durable receipt before the
  epoch is checked, so a retry after a restart is answered too (KN5.21 to KN5.23, LI.6, LI.30).
- Program jobs run one exported function of a loaded program on opened snapshots, on the exclusive lane (one job) or
  the program lane (its declared capacity, 2 by default), under a declared budget with the test tariffs. The job's
  host answers four whole-state queries on its inputs (`inputs`, `state`, `moved`, `diff`) and grants nothing else. A
  `diff` answer is cached, and a hit is charged as a computation (KN7.8). A commit, an undo, a redo or a switch cancels
  the queries that read the replaced revision; file and pinned jobs finish (KN7.5, LI.25). A result is bound to its
  first input's digest, and `job_result_at` refuses a revision with another digest (KN7.3).
- Tests (`c600-session/tests/`), each naming the conformance tests it covers:
  - `store.rs`: CT-KN4-02 at every write of seven kinds of durable action, with three kinds of failure, with and
    without a crash, and the unproved fate with each kind of failure at the append and at the sync; CT-KN4-03;
    ONE.70's torn and damaged tails; a session on a directory.
  - `session_lock.rs`: the session lock. A second opener of a held session is refused with
    `KN4:busy.session_locked` and changes no file. That holds for all four openers (`Session::open` and `create`,
    `JournalStore::open` and `create`), whether the holder is in the same process or a child process. The
    session opens once its holder is dropped, or once the child is killed (Fable lead L9).
  - `commit.rs`: CT-KN5-01 with PHS.19's preview-swap row, CT-KN5-04, CT-KN5-06, CT-KN5-11 and LI.33's gate-order
    trace; KN5.9 to KN5.11; tokens, the row and the revisions a token binds, and the rows a program may deliver; the
    commit lane under a held write; outcome lookup across a reopen.
  - `crash.rs`, from the Fable crash-consistency task: a reopen syncs a frame before it serves it; an append torn or
    zero-filled at every byte of each kind of action is wholly before, writable and cut durably, and a whole one is
    wholly after; every prefix of a history over the seven kinds opens at its last whole frame with its receipts; a
    damaged byte anywhere in it opens read-only at the frame before and is never cut, and in the header open and
    `create` refuse; a failed recovery is done again whole; a lost, emptied or damaged state object is replayed and
    written again; a retry after a restart gets the durable receipt; `create` is honest about the header; and seeded
    histories over the seven kinds, refusals included, replay with the same view, receipts, record and checkpoint
    contents and undo chain.
  - `branches.rs`: CT-KN4-04; snapshots by revision and by checkpoint.
  - `jobs.rs`: CT-KN7-01 to CT-KN7-05 and CT-KN7-07 with PHS.19's budget row (five budget cases, each run cold, warm
    and after an eviction); CT-KN5-09; CT-KN4-01's in-process part: a job's file read or write, workspace write or
    proposal is refused by its host without touching the store, and a scan of `c600-run`'s source finds no
    `std::fs`, `path`, `env`, `io`, `os`, `net` or `process` path.
- Drafts, which freeze with chapter 01's open rows, the command table (LP3) and the persistence encodings: the row
  names, record shapes and Rust shapes; the journal format, which is not the frozen migration format; the token check
  order (verify, then unused, then the commit lane, so a token is used up even when the lane is busy); a program
  delivers job rows only (another commit row gives `KN5:capability.commit_row_refused`, another session row
  `LG3:capability.not_granted`); a read-only session refuses proposals too; undo, redo and a switch must pass the
  protection in force; a token for a row without a preview whose `state` or `protection` revision moved gives
  `KN5:stale.revision_changed`, and an execute token whose word or dependencies are not the pending preview's gives
  `KN5:stale.token_mismatch`; the shell stub mints under the session core, so it waits for a commit write in flight;
  the torn-tail rule above, under which a hole inside a frame is damage; a lost state object is written again by the
  next checkpoint of its state, and its checkpoint opens by replay until then; `create` on a disk that holds a frame
  gives `PS1:input.persistence_format`; a checkpoint advances the workspace counter and is not undone; a workspace
  receipt's result digest is the SHA-256 of its frame; a snapshot of another epoch, given as a job input, gives
  `KN2:stale.epoch_changed`; stopping an unknown job gives `KN5:stale.result_gone`; an I/O error at open gives
  `KN4:resource_limit.disk_full`; the first interruption of a job is the one it reports; the four job queries and
  their kernel costs (one unit per 1,024 slots for a whole-state query); a result's coverage is its inputs' revisions
  and the meter's summary. KN5.12 stays open: only the shell proposes in M6, and its proposal replaces the pending
  preview.
- Gaps: the secret and the 64-bit epoch number come from std's randomly keyed hasher, the time and the process ID,
  not from the operating system's generator; `FsDisk` syncs a directory entry on Unix only, and it opens a new handle
  for each append, cut and sync, so on Linux a sync may miss a write-back error that the writing handle would see,
  and on Windows a scanner that holds the journal without sharing can fail a commit and its rollback alike (Fable
  lead L8); there is no historical workspace version (a snapshot of an earlier revision holds its state only); a
  proposal takes
  literal letters or a record of letters only; the admission and the end of a durable row share one frame; the IDs of
  proposals, job rows and refusals are not remembered across epochs, and the receipt of a refused or failed durable
  row lasts for its epoch; undo, redo or a switch that would replay more than 100,000 letters is refused; the core
  lock is held during disk I/O, a checkpoint's read of its object included, so a status read waits for a write in
  flight; a journal cut inside its last frame by outside damage, or with zeros written over the end of that frame,
  reads as a torn append and is cut at open.
- Not run or not applicable, which is no pass (PHS.63): step 7 of the commit path
  (`KN5:consistency.state_hash_mismatch`) is never reached by a test, because step 6 refuses every head change first;
  `KN7:resource_limit.runtime_lost` is tested through the terminal mapping only; the commit lane's refusal is tested
  with a held write only; the seeded replay histories run in debug at a reduced scale, four histories of 16 actions
  where the Fable lead asked for 200 of 40, and the crash tests open sessions at sampled offsets while their scans
  cover every offset; L8's Windows sharing test and L9's two-opener test are not written; L10's job row has no durable
  record to recover in M6; and the rows the plan leaves out of M6 (section 10).
- Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the store with the
  acceptance check. Its one major finding was fixed before the pull request, and a scoped re-check found no blocking
  finding: R1, a line switch's token bound no revision, so it committed after the state had moved. Fixed: R2, a
  checkpoint's lost object (above); R3, a zero-filled tail read as damage; R5, a genuine token of another row gave the
  wrong code and now gives `KN5:stale.token_mismatch`; R7, a job count raced a finite job; R8, damage was injected in
  the last frame only, and no partial write met a failed rollback. Comment added: R4, a refusal at a busy commit lane
  uses the token up, as chapter 08's step order has it. No action: R6, a read-only session refuses proposals (a
  draft, above); R12, `state_from_labels` checks a permutation only, and its one caller checks the digest. Deferred:
  R9, a poisoned lock is recovered silently, with no reachable panic; R10, the core lock is held during I/O (a gap,
  above); R11, a caller can forge a snapshot handle's public revision fields, which a program cannot reach; R13, the
  scan for file-system calls looks for `std::` paths only. The re-check's three minor notes are recorded above: the
  stub's mint waits for a commit write (a draft), a checkpoint reads its object under the core lock, and zeros over
  the end of the last frame read as a torn append (gaps).
- Leads from the Fable crash-consistency task (owner decision, 15 November 2026), each now a test in `crash.rs` or a
  gap above: L1, a reopen served a receipt for a frame it had not synced; L2, a zero-filled tail left the session
  read-only for good; L3, a checkpoint completed while citing a lost object; L5, an identical retry after a restart
  was refused as stale; L7, `create` reported an unknown header fate as a failure and wrote to a disk it then refused.
  All five are fixed. L4 and L6 became the seeded replay test and its oracle (the durable view, receipts, contents and
  the undo chain), and L10's rows 1 to 7 are covered. L9 is fixed by the session lock (above); L8 is deferred.
- KN5 (LP2): `JournalStore` (`src/command_store.rs`) implements c600-command's `Store` and `Reopen` over the journal,
  in a draft frame format of its own (`c600-command-journal-draft-1`, not the frozen migration format), so the command
  layer runs durably on this crate's disks; the dependency runs from this crate to c600-command only. Each record
  value an action writes is a content-addressed record object, `objects/<sha256>` of its canonical bytes, written and
  synced before the frame that cites it, so FP.9's 64 MiB record bound holds through the journal: a larger value is
  refused with `PS1:resource_limit.persistence_size` before anything is written, the frame's other fields stay under
  its 16 MiB draft bound, a missing or altered object ends the history in read-only recovery (KN5.23), and a journal
  written before record objects still reads its inline values (pull request 388). Tests: `tests/kn5_command.rs` and
  `tests/record_objects.rs`. M6's `Shell`, `Session::invoke` and `Session::shell` are superseded by that layer and
  are removed once their tests run against it.
- M6's own `invoke` keeps the layer's ruling 4: the program gates and the authenticator run before an envelope takes
  its command ID, and their refusal is kept for the epoch under the command ID and the request digest without taking
  the ID, so an envelope with a bad authenticator, or a program's commit or non-job row, sent first under the shell's
  next command ID no longer makes the genuine press `KN5:input.command_id_reused` (`tests/kn5_command.rs`). Until it is
  removed it still differs from the layer's KN5 rulings: it stores `accepted` under the command ID before steps 5 to 8
  (ruling 3; `commit.rs` asserts it during a held write); its request digest includes the origin and the
  authenticator (the layer's excludes the authenticator, LI.5); command IDs are one space for the shell and programs;
  an `epoch_changed` refusal is not kept; and its shell stub mints for a fresh preview that need not be presented
  (LP3's L4). Under the owner's host rule of 16 November 2026 (PHS.52 ruling 4) its envelope names no origin:
  `Session::invoke(origin, envelope)` takes the origin of the channel the envelope came on, and the request digest
  and the program gates read that origin (`tests/kn5_command.rs` sends the shell's genuine envelope on a program's
  channel first).
- The record layer (`src/persist/`, M6's remaining rows) reads and writes chapter 05's records over the frozen format
  packet: FP.5's canonical bytes, FP.7's deterministic gzip and FP.9's limits (`codec`); FP.4's catalogue with
  `read_record` and `write_record`, where an unknown or newer format, version or required extension stays recorded
  and never activates (`record`); FP.13's flat menu words with their chunks, annotations and X3's expansion bound
  (`word`); FP.19's facts envelope, read into `c600-evidence`'s graph, with the display word for each recorded
  function edition (`facts`); FP.29's migration record, the function-edition change and FP.28's publication, which
  validates a whole generation before it writes anything, writes it durably and only then appends the active pointer
  (`migrate`); 0.4's preview certificate as a recorded claim (`legacy`, PS3.7); and the run record (`run`, PS1.31):
  chapter 09's `ReproKey` with every DET.1 field, the consulted inputs captured by digest before the key is sealed,
  the grants, the profile and the policy identity, and the exact payload. A run record reads DET.30's coverage, counts
  a published witness once a check record confirms it (KN7.4), and admits a reproduction before execution, which
  stays the caller's: LP3's runtime in a session, the interpreter in the tests. A stored menu word is admitted into a
  session only under the session's whole identity block, and with Jumble off only with retained letters
  (`word::admit_in`). FP.38's `c600-pick` record (`pick`, LP2's pull request 359): its payload `{list, fixed, options,
  chosen, target}` is checked when the store writes it, and a block's pick resolves in FP.10's order. The layer's
  draft choices are listed in `src/persist/mod.rs`. The session's journal does not store through this layer yet; it
  keeps its own draft frames (above), and the jumbling episodes are KN5's records in them.
- Tests: `persist_ps1.rs` runs CT-PS1-08 to CT-PS1-10, CT-PS1-19, CT-PS1-21 and CT-PS1-23 whole, and `persist_ps2.rs`
  runs CT-PS2-01 to CT-PS2-05 whole, over the packet's vectors read in place (shared helpers in
  `tests/persist_support/`). The publication runs on `MemDisk`, with each write failing in turn and with an outage
  from each write on, so a pass is no power-loss evidence (FP.28). `persist_run.rs` runs CT-PS1-16, CT-DET-04,
  CT-DET-14 and CT-BND-23 whole: the test parses, checks and runs each program on RT1's interpreter and plays the
  executor's half that LP3's runtime plays in a session (capture, seal, run, `run::exact`). `persist_jumble.rs` runs
  CT-PS1-11 and CT-PS1-22 whole by the union of their parts: stored menu words and the journal's replay under the
  I_a menu for CT-PS1-11; the never-on journal, and a failure at each append of an entry, a pose move and an exit
  with a power loss after it or after the step sent again, for CT-PS1-22, on `MemDisk` again. `persist_picks.rs` runs
  FP.38's vectors, the payload rules and the admission of the vectors' picks.
- Critical-path read of the record layer (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model
  read `src/persist/` and its tests with the acceptance check. Its two major findings were fixed before the pull
  request: R1, a publication after a torn pointer line wrote a line feed first, so a failure after that byte completed
  the torn line and activated a generation never reported as published; a publication now cuts a torn tail durably
  before it appends. R2, a migration took its source's format, version and identity on trust; a `c600-` source must
  now carry them in its bytes. Also fixed: R3, a migration's check records are published with it; R4, a generation's
  head and validation record are read with `read_record`, so a newer one never activates; R5, a damaged last pointer
  line is refused, not skipped for an older one. Each fix has a case in `persist_ps2.rs` that fails without it.
  Deferred: R6, a conversion that changes a word's extensions without a note is not refused, since PS2.5's preserved
  fields do not list them. A scoped re-check found each finding fixed and no new blocking finding. Of its three minor
  notes, one is fixed: a `c600-` record given under another format name is refused. Deferred: a damaged last pointer
  line stops every later publication, and the store has no repair path; `reconcile` reads a missing pointer log after
  a failed append as no switch, which holds while a failed append leaves no unreadable file.
- Critical-path read of the run record and the jumbling tests (second pull request): a fresh Claude subagent on a
  smaller model read `src/persist/run.rs`, the word admission, `persist_run.rs` and `persist_jumble.rs` and found no
  blocker or major. Its minor findings are fixed: a result whose outcome and code disagree is refused; a key names
  each module once; a check that reads open itself, as under a clock grant, confirms no witness; `reproduce` says
  that a record that cannot repeat answers `not run` before admission; a seeded run reproduces from its seed, fetched
  by digest; and a step sent again after a rolled-back append commits and survives the power loss. Each source fix
  has a case that fails without it. Deferred: a reproduction with a `pick` or `provenance` capture, since CT-DET-04
  names a workspace, a protection input, a grant and a file. Two review notes on the first pull request are fixed
  too: a stored word length past `i128::MAX / 2` is refused instead of overflowing, and `record::follow` says that its
  caller checks the expected kind and the Ref's format and version, as the packet's reference does.
- Evidence kind: source and fixture. Failures are injected in memory; `FsDisk` runs in one reopen test and the
  session-lock tests, so a pass is no evidence of a real disk's behaviour in a crash. The checks ran on Windows
  only.

## Status of the evidence library: chapter 06

- `c600-evidence` holds chapter 06 as pure code. It reads and writes no file, clock, process or session (EVD.54), and a
  word is a function of the graph's facts and a pinned edition, `evidence-draft-1` (EVD.14). Where chapter 06 leaves an
  encoding or a detail open, the crate takes a draft, and its documentation (`c600-evidence/src/lib.rs`) lists each
  one; none is a decision of the chapter. The trial threshold stays the owner's (EVD.29), so the tests use the trial
  set and its up-subsets only.
- Tests (`c600-evidence/tests/`): `words.rs` runs the word function's rows and checks (CT-EVD-01, CT-EVD-05,
  CT-EVD-07 to CT-EVD-11, CT-EVD-13, CT-EVD-22); `properties.rs` runs seeded random graphs (CT-EVD-02 to CT-EVD-04,
  CT-EVD-06); `sinks.rs` runs writers, reuse, check reports and the import rules (CT-EVD-17 to CT-EVD-21, CT-EVD-26 to
  CT-EVD-30, and the evidence halves of CT-LB6-02, CT-LG5-06 and CT-PS1-27); `examples.rs` transcribes section 9's
  E1 to E3 (part of CT-EVD-12).
- Fixed, each found by a test: I6 skipped a statement with no OR node of its own, so a bad refutation of an unsupported
  universal read row 1 instead of row 0 (CT-EVD-09); EVD.42's guard compared a leaf's covered set and not the search
  record's declared set, from which EVD.59 takes it, so E3's search refuted its statement instead of reading row 0
  with `inconsistent_fields` (CT-EVD-12).
- Open, across chapters, with no frozen text to change: FP.20 carries a search record only inside a leaf, but EVD.32's
  row 5 and E3's truncated beam and cancelled runs need one for a key with no leaf, so the crate attaches it to the
  key's OR node as a draft; and a checked witness of an existence statement also offers a refutation of its universal
  negation, so without scope membership both words read row 0.
- Not built: the label-table importer and its corpus walk (EVD.48 to EVD.51), the FP.19 and FP.20 wire encoding, KN6's
  checkers (the library only admits their reports), EV5's display and teaching, the independence display (EVD.28),
  PS3.12's dispatch, obligation export (LB5.18), storing the envelope (chapter 05), the command layer, the runtime
  process and the jumbling half.
- Evidence kind: source and fixture, with synthetic graphs. The checks ran on Windows only.

## LP2 conformance tests (chapter 12, section 15.3)

Section 15.3 lists 248 conformance tests for LP2. A test names each ID it covers: `(whole` when it runs the whole
definition, alone or with the tests it names, and `(part: ...; not run: ...)` otherwise (PHS.62). A test that is not
run or not applicable is no pass (PHS.63).

- Where they are:
  - `c600-check/tests/`: the checker's own tests carry their IDs (`effects.rs`, `exactness.rs`, `mismatch.rs`,
    `vectors.rs`, the built-in column's `built_in.rs` and its exceptions' `exceptions.rs`, M7 attack leads' `leads.rs`
    and M1 limits' `limits.rs`).
  - `c600-syntax/tests/`: `conformance_lg1.rs` to `conformance_lg3.rs` write the LG tests in source; `errors.rs` holds
    CT-ERR-05's encoding half (ERR.10's raw context for each malformed shape); `syntax.rs`, `format.rs` and
    `lowering.rs` mark M7's tests.
  - `c600-run/tests/`: `conformance_lg2.rs` to `conformance_lg4.rs` run the LG tests (shared helpers in
    `conformance/mod.rs`); `conformance_bnd.rs` runs the boundary tests, and `tables.rs` enumerates the grant table
    and the corpus; `legacy.rs` runs CT-PS1-18's legacy parts; `conformance_det.rs` runs the CT-DET rows on RT1;
    `canonical_ps1.rs` runs the canonical-profile rows (CT-PS1-01 to CT-PS1-07, CT-DET-23 and, synthetic, CT-KN3-02);
    `traces.rs` and `values.rs` mark the meter's and the values' rows; `det65_programs.rs` runs 32 of DET.65's 40
    traces as programs and lists why the other 8 do not run; `decode.rs` runs DET.62's byte decoding; `streams.rs`
    DET.63's streamed words; `generators.rs` runs DET.61's generators,
    `child_scopes.rs` DET.60's child scopes and `loops.rs` DET.55's loops.
  - `c600-kernel/tests/`: `kernel.rs`, `load.rs` and `json.rs` run the 600-cell-Full halves of the KN rows, CT-PS1-04
    and CT-PS1-12 (M4's section).
  - `c600-jumble/tests/`: `crossing.rs` runs CT-KN2-03, and `assets.rs` the jumbling model's asset checks (M5's
    section); `domain.rs` and `refusals.rs` hold the candidate's halves of Fable's M5 attack leads; `kn3.rs` runs
    KN3's exact rotation requests under A6 (CT-KN3-05 whole; CT-KN3-04, CT-KN3-06, CT-DET-16 and CT-LG2-16's kernel
    half in part), with `c600_kernel::exact`'s unit tests for CT-KN3-01's part; `observations.rs` and `levels.rs`
    run M8's observations and its levels, end tests and derived values, `nu.rs` runs ν and `corrections.rs` the
    correction catalogue (M5's section).
  - `c600-evidence/tests/` runs the EVD rows (the evidence library's section).
  - `c600-module/tests/conformance_lg5.rs` runs CT-LG5-01 whole and CT-LG5-02 to CT-LG5-07 in part against LP3 C4's
    loader and lockfiles. The parts leave out the boundary class against chapter 07's reviewed classes, the run
    record, the reproduction key and a replay from them (M6), a check at the run itself, EV1's active-version
    bookkeeping, evidence, the classification of forks, and a real module signature; each marker names its own.
    CT-LG5-06's evidence half runs in `c600-evidence/tests/sinks.rs`, so its two parts together run it whole.
  - `c600-session/tests/`: M6's tests mark their rows (M6's section), CT-ONE-03 to CT-ONE-05, CT-ONE-11, CT-ONE-21,
    CT-ONE-23 and CT-ONE-24 included. `conformance_rows.rs` runs the session's half of CT-ONE-01 and CT-ONE-02 over
    every `Row` variant, CT-ONE-23 and the session's rows of CT-KN5-03 and CT-KN5-10; `conformance_jobs.rs` runs
    CT-ONE-06, CT-ONE-07 and CT-ONE-12 for an execute, an undo, a redo and a line switch; `conformance_recovery.rs`
    runs CT-ONE-08, CT-ONE-11's refusals at each reachable step, CT-ONE-20, CT-ONE-21 with ONE.17's stored
    references, and CT-ONE-22. `c600-command/tests/layer.rs` names LP3 C1's parts without a marker (the C1 section).
  - Python, listed in `tools/ci/run_headless.py`: `tests/test_error_registry.py` checks the code catalogue against
    chapter 13 and the LP1 edition fixture `tests/fixtures/error_registry/lp1-edition.json`;
    `tests/test_language_scans.py` scans the workspace's sources; `tests/test_format_inventory.py` runs CT-PS2-06;
    `tests/test_source_checkout.py` reads every `.6cs` source and every pinned packet file as LF, CRLF and `input`
    checkouts write them (CT-LG1-02, CT-PS1-05's checkout half); `tests/test_kernel_oracle.py` replays O01 to O07 and
    N01 to N04 through the kernel (M4's section); `tests/test_jumbling_rust_candidate.py` compares the jumbling
    candidate with J1 (M5's section); `tests/test_jumbling_rust_attacks.py` runs Fable's M5 attack leads, with J1's
    faults as expected failures (M5's section); `tests/test_jumbling_rust_m8_observations.py` and
    `tests/test_jumbling_rust_m8_levels.py` compare M8's observations and levels with J1,
    `tests/test_jumbling_rust_nu.py` compares ν with the ν reference, and `tests/test_jumbling_rust_corrections.py`
    the correction catalogue with the research library (M5's section).
- Count with LP3's C1 to C8, M5, LP2's first and second integration batches (KN3's exact requests among them), KN5's
  stages 1 to 5, its overlay availability and its report after a command, the Linux Rust job, M8's levels and M6's
  record layer with its run record and jumbling records, the stage checker, whole-state contracts, the builder, streamed
  words, `filter.apply`, PHS.19's items 7 and 8 (CT-KN2-02's remaining states, R-11's jumbling fixtures), LP2's exit
  re-marking pass and the command-layer tests of the owner's reading 11, as `tools/language/ct_index.py --summary`
  prints it (`tests/test_ct_index.py` checks the tool): 126 whole, 32 of them by the union of their parts (CT-BND-14,
  CT-BND-27, CT-ERR-09, CT-EVD-21, CT-KN1-01, CT-KN1-04, CT-KN1-05, CT-KN2-01, CT-KN2-04, CT-KN2-05, CT-KN5-02,
  CT-KN5-12, CT-LB7-02, CT-LG2-04, CT-LG2-12, CT-LG2-13, CT-LG2-14, CT-LG3-01, CT-LG3-02, CT-LG3-04, CT-LG5-06,
  CT-LG5-08, CT-LI-04, CT-LI-06, CT-LI-07, CT-LI-13, CT-ONE-01, CT-ONE-02, CT-ONE-03, CT-PS1-11, CT-PS1-22 and
  CT-RPI-01); 121 part; none named only; none missing; 1 waiting (CT-LI-14), with what it waits for in
  `docs/progress/1.0/lp2-ct-index-notes.md`. That is the count on LP2's exit candidate `0260e6b1`. With the owner's
  readings of 22 November 2026 in those notes (LP2 plan section 12), the tool prints 126 whole (32 by union), none
  part, 7 not applicable, 8 waiting and 107 indexed elsewhere; none of these 122 is a pass (PHS.63). After LP2's exit,
  reading 18's undo fix (pull request 416, in LP3's pull request 417) makes CT-LG3-03 and CT-LB7-01 whole by union,
  and the owner's exit sign-off of the same day moves reading 17's owner-machine halves to the post-freeze on-screen
  line (its reading (a)): the tool now prints 128 whole (34 by union), 7 not applicable, 8 waiting and 105 indexed
  elsewhere, and on `0260e6b1` still the exit count.
- The headless Linux legs of CT-DET-02, CT-DET-15 and CT-PS1-01 run in the Linux Rust job
  (`.github/workflows/linux-rust.yml`, owner decision of 16 November 2026), against the same pinned bytes as the
  Windows shards, so CT-PS1-01 is whole. The Python comparisons with J1 run there too, in the J1 job (PHS.19's
  cross-platform item). These are the six `tests/test_jumbling_rust_*.py` checks that the Windows shards run
  (candidate, attacks, M8's observations and levels, ν and the correction catalogue), one runner each. They use the
  same pinned CPython and hash-checked NumPy wheels and build the release adapter offline. Each system compares the
  candidate with J1 on the same pinned inputs and keys. The two systems' payloads and stop reasons are compared byte
  for byte by the LP2 workflow (`.github/workflows/lp2-full.yml`, nightly on main and on request): ten checks (the
  kernel's oracle replay, the LP3 loop and source route, the jumbling command route and the six J1 comparisons) run
  on windows-2025 and on ubuntu-24.04 with `C600_TRANSCRIPT_DIR` set and keep their exact outputs
  (`tools/ci/transcripts.py`), and a third job compares them. The same workflow runs the J1 comparisons that read
  `JUMBLING_FULL_REPLAY=1` on every record, and `jumble_on_layer.rs`'s whole fixtures (`--ignored`) in a release
  build. A pass is headless Linux and Windows source and fixture evidence only.
- Not run or not applicable, which is no pass (PHS.63), with the check that runs instead: CT-LG3-06's flow-label
  variant, while the flow rule is off (BND.41) and LQ2 has not set it, so the static-only and run-time variants run
  (`c600-run/tests/conformance_lg3.rs`); CT-LG1-03's second host context, since the editor services that host the
  formatter come with LP4; CT-LG4-01's second admitted route, since RT1 is the only one (LQ9); CT-BND-11's ONE.32
  fields in other profiles, which never arise (BND.60); form 2 in CT-DET-32, since RT1 runs form 1 only (the LP2 plan's
  selection, which the owner may overrule); ERR.1 for the six codes `c600-command` named as drafts until LP3's
  command-table freeze registered them (`tests/test_language_scans.py`'s `UNREGISTERED_DRAFTS` is empty since); the
  parts of the session's rows that their markers leave to LP3's view, display, command layer and runtime process, to
  A6's live twist and to M5's crossing, and step 7 of CT-ONE-11, which M6 cannot reach (M6's section); and, as
  before, CT-LG1-01's mixed dependency editions and CT-LG2-05's form 2.
- Fixed, each found by a test:
  - A clock reading was an exact `Nat`, so `use_budget(read_clock())` was admitted against LG2.6, LG3.8 and DET.18; it
    is now `Display<Nat>`, and CT-LG2-06's seconds half runs. LP3 C4's clock probe P01
    (`c600-module/tests/ct_rte_07_full.rs`) now returns `Display<Nat>`, so it still reaches the grant check.
  - The provenance reader accepted a 65th nested container when the innermost one was empty, above FP.9's json_depth
    of 64 (CT-PS1-07). A fresh Sonnet read found no blocker or major finding.
  - The kernel's adapter refused a whole case when one recipe held JSON that 0.4 refuses step by step (L1), and
    `Kernel::letters` expanded another model's recipe with this model's seeds (KN1.4); M4's section has both.
  - The evidence library's two fixes (its section).
  - `open_snapshot` refused a `RevisionRef` stored in an earlier engine epoch with `KN2:stale.epoch_changed`, though a
    different epoch does not invalidate a read (ONE.17, CT-ONE-21). It now opens the same revision under the new
    epoch; a handle of the old epoch is still no job input, and an event the store no longer holds is
    `KN2:stale.revision_gone` (`c600-session/src/session.rs`). A fresh Sonnet read found no blocker or major finding.
    Its minor points need no change: an epoch the store never had is not refused, which the rest of the reference
    makes harmless, and after damage a reference whose counters lie past the surviving frames is gone even when its
    head survived, which refuses rather than opens.
- Exact sinks (owner, 16 November 2026, PHS.52): the checker takes only portable exact data at `use_goal` and
  `use_evidence` (M1's section), and the interpreter encodes the value with DET.59's priced encoding pass (an `encode`
  row before the `use_exact` row) and hands the host the canonical bytes, so a function or a receipt never reaches the
  host (RPI.54, RPI.1); a value with no canonical form stops the run with `PS1:input.noncanonical_value` before the
  host is called. Receipt-as-evidence is a question for the next milestone card.
- Chapter 13's amendment (owner, 16 November 2026, PHS.52): chapter 13 changed once, in the amendment LP3 made at its
  command-table freeze (PHS.53). It took the ten draft codes `c600-command`
  emits (`KN2:consistency.star_certificate`, `KN5:input.unknown_row`, `KN5:input.argument_form`,
  `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict`, `KN5:consistency.unadmitted_journal_action`,
  `KN5:capability.preview_shown`, `KN5:capability.outside_solver_command`, `KN5:input.saved_set_exists`, and KN5's
  `KN4:input.session_exists` for a store that already holds a session; `tests/test_language_scans.py` keeps the exact
  list), M5's section's four drafts, DET.36 in the rule source of `LG3:capability.not_granted`, and
  these errata: in `13-errors.md`, the LB6 closure row `evidence.decomposition_mismatch` (line 661) sits between
  ERR.28's table and ERR.31 and belongs at the head of ERR.31's table, and ERR.31 comes before ERR.29 and ERR.30; the
  rule source of `KN5:stale.filter_preview` (line 322) omits KN0.9 (`closure/manifests/p7-layers.json`); the Used-by
  cells of `KN2:input.recipe_shape` and `KN2:resource_limit.primitive_ceiling` say "not yet used", but the kernel emits
  both, and the cell of `KN2:input.word_blocked_prefix` says the same, but the jumbling adapter emits it for a refused
  word (M5) and M8's `derive` for a refused letter. LP3's command-table freeze made the amendment (chapter 13,
  section 9), with the five later codes the owner added on 20 November 2026
  (`../docs/wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md`, item 6).
- Open, each needing a decision:
  - ERR.6: `c600-kernel/tests/kernel.rs` compares refusal texts, because `c600_kernel::Refusal` has no typed context
    (CT-ERR-16's typed failed fact); `c600_syntax::LowerError` has no code.
  - For LP3: LB7.1's `LB7:capability.module_grant_missing` is the workspace library's entry-admission fact, built with
    LB7; the checker's own admission reports `LG3:capability.not_granted` or `LG3:capability.ambient_read` (PHS.52),
    as chapter 02's LG3 table has them.
  - For LP3, from the LG5 tests (`c600-module`): a built-in approval of a closure built for another IR version is
    refused with `LG4:input.ir_rejected`, because the built-in branch looks up the function summary first, while
    the own path gives `LG4:input.ir_version_unsupported`; the manifest's `boundary` is checked for form only; a run
    request names no session identity; the caller supplies the author class of a fork or an import; approvals live
    only in memory, with public fields.
  - KN5 has one implementation: LP3's `c600-command`, durable over M6's journal through `c600_session::JournalStore`
    (KN5's stages 1 and 2; LP3's C7 section below). The CT-KN5 parts that run, run against it; M6's `Session::invoke`
    follows ruling 4's order and the host rule until its tests move to `c600-command` (M6's section).
  - `Kernel::protection` itself checks no model; `Kernel::checked_protection` adds KN1.4's check (M4's section).
  - For M6's store: the kernel's `archive::gunzip` accepts concatenated gzip members and trailing zero padding, so
    FP.7's strict one-member reader must refuse them itself; run-time `encode` applies no json_depth or integer_digits
    limit, so the store's writer must check FP.9 before it persists a value.
  - Gaps: the interpreter does not check a function's kind at run time; `provenance::import` makes the author `own`,
    so `Author::Imported` never arises (FP.18's example has `own`); `Loaded::run` does not require the entry function
    to be exported, and `load` does not run the checker again.
- Waiting (1): CT-LI-14, an output-exhausted `jumble.observe`, whose row no crate builds; LP3's command-table freeze
  builds or drops the row. Every other ID runs at least in part, and each part names what it leaves out.
- Evidence kind: source and fixture. The checks ran on Windows only.

## Status of LP3, C1: the command layer

- `c600-command` runs every change to a session through `CommandLayer::invoke` and its gates (KN5.1) over H-08's
  stored table (`docs/progress/1.0/command-table.json`, 174 rows). A word commit runs chapter 08 section 7's steps in
  order: the shell's gate and token (steps 1 and 2), the authenticator and the unused nonce (3), commit-lane
  admission (4), the token's binding and its presented preview (5), `fresh(p)` (6), the head hash (7), the protection
  recheck (8), the durable transaction (9) and adoption (10). `session.undo` and `session.redo` take LI.9's hint and
  derive through the kernel under the protection in force (LI.26). Every invocation gets a receipt (KN5.21); a command
  ID the layer knows is answered by its outcome lookup (KN5.23, LI.30), across a restart too.
- The shell mints the user-action tokens from a per-launch secret the runtime never receives (KN5.14, KN5.15) and
  resolves the draft keymap's chords before the envelope (EV4.3), so a key bound to a kernel row reaches the engine
  with no interpreter step (ONE.48). The runtime reaches at most a preview: the 19 commit and app rows and any token it
  carries are refused (ADR-L02).
- Tests (`c600-command/tests/layer.rs`): parts of CT-KN5-01, CT-KN5-03, CT-KN5-04, CT-KN5-06, CT-KN5-07, CT-KN5-09,
  CT-KN5-11, CT-KN5-12, CT-KN7-04, CT-KN7-05, CT-ONE-03, CT-ONE-04, CT-ONE-05, CT-ONE-11, CT-ONE-20, CT-ONE-21,
  CT-ONE-22 and CT-LI-10, each naming its part. These tests are indexed in LP2 (chapter 12 section 15.3); they count
  as run only when their whole definitions run.
- `tests/test_lp3_loop.py` builds `c600-lp3-oracle` in release and drives O01 to O07 through the command layer on the
  command route and the key route: `tools/oracle/compare.py` reports every step equal with no omitted fields (3,201
  steps per route), every accepted step commits once, independent cases undo each commit, chain cases undo and redo
  every commit onto the recorded states, and the two routes write byte-identical traces and receipts. A tampered trace
  fails at its step, and a case with another model identity is refused without a trace. `tools/ci/run_headless.py`
  lists it.
- Drafts as of the command table's freeze at LP3 (PHS.53): the list in the crate documentation, among them one
  session per layer with Jumble off, an inline recipe as `WordRef`, `twist.apply` refused as a row not built (KN3's
  exact rotation requests exist since LP2's pull request 176; the row is KN5's), five codes chapter 13 did not register
  until that freeze (its section 9), and the envelope's epoch (LI.5).
- Not built: the durable transaction is `MemoryStore`, a stand-in until M6 is on main, so no receipt here is
  persistence evidence; the source route, the views and the other two tools (LP3 plan, C2 to C8); protection rows;
  every row the crate refuses with `KN5:input.row_not_built`.
- Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the crate with the
  acceptance check and found no blocker. Fixed before the pull request: R1 (major), a retry after a restart was refused
  as `command_id_reused` because the request digest used the layer's current epoch, not the envelope's; R2 (minor),
  the token's binding was checked before commit-lane admission; R3 (nit), the outcome lookup ignored the request
  digest. A scoped re-check found all three fixed and no new blocking finding.
- KN5 over M6 (LP2): `CommandLayer::open` replays a store's history through the kernel and restores the revision, the
  protected set, the undo and redo stacks and the durable receipts. `c600_session::JournalStore` is that store over
  KN4's journal and `MemoryStore` keeps its contract in memory; this replaces the not-built note on the durable
  transaction above. Every open and every restart draws a fresh random epoch the session never had (L10).
  `protection.orbit.set` is built, and undo and redo walk M6's move tree, a protection change included, with
  `confirm` to release an orbit (LI.26). A workspace overlay's completion is durable with its receipt (KN5.23). An
  action that does not move the puzzle keeps the state revision, the preview and the jobs (KN5.18). The layer's test
  hooks exist only in debug builds or with the `test-hooks` feature (L15).
- KN4.3's journal query (LP2's pull request 383; the drafts are in the crate documentation).
  `CommandLayer::journal(from, limit, cursor)` pages the move tree newest first, from the head or a named event along
  its parents to the start, on that event's line only. Each `JournalEntry` gives the event, its `EntryKind` (start,
  commit, crossing, pose or exit), a commit's retained letters, whether the start is solved and an exit's mark. A
  `JournalPage` holds 1 to `JOURNAL_PAGE_LIMIT` (4096) events; its cursor binds the session, the walk and the next
  event, and grants nothing. The read replays nothing and changes nothing. Tests: `c600-command/tests/journal.rs`.
- KN5 tests (`c600-command/tests/kn5.rs`, from the Fable KN5 attack leads L3 to L11, L15 and L18): parts of
  CT-KN5-01, CT-KN5-04, CT-KN5-09, CT-KN5-10, CT-KN5-11, CT-ONE-03, CT-ONE-04, CT-ONE-05, CT-ONE-11, CT-ONE-21 and
  CT-LI-09. `c600-session/tests/kn5_command.rs` runs the layer over the journal: parts of CT-ONE-21, CT-ONE-22,
  CT-KN4-02 and CT-KN5-04, and CT-KN5-04's attack against M6's own `invoke`. Each test names its part; none runs a
  whole definition.
- KN5 drafts (listed in the crate documentation): ruling 2, an undo checks its hint before its bound revisions, so a
  raced undo is `KN5:stale.undo_superseded`; ruling 3, no receipt or lookup answers `accepted`; ruling 5,
  `KN5:auth.token_invalid` for a failed authenticator and `KN5:stale.token_mismatch` for another session or epoch;
  ruling 7, `session.checkpoint.save` follows the table (a commit row with a token; not built), where M6's
  `session.checkpoint` takes none; `KN4:input.session_exists`, a sixth code chapter 13 did not register until LP3's
  command-table freeze (its section 9).
- Owner decisions of 16 November 2026 under PHS.52 (`../docs/wiki/decisions/owner-decisions-2026-11-16-phs52.md`).
  (1) Ruling 4 (KN5.21, KN5.23) keeps the frozen order, the retry lookup before the token checks (chapter 01 KN5,
  chapter 08 section 7, LI.6), and the host stamps the caller from the channel the envelope came on and never takes
  it from the envelope: `Invocation` names no caller, and `CommandLayer::invoke(&caller, invocation)` takes the
  channel's. Command IDs are scoped per caller (the shell, and each run on its own since finding G1's reading) and the
  request digest covers the stamped caller; a refusal of an
  envelope that does not authenticate (a bad authenticator, a missing token, or a caller that may not use the row or
  hold a token) is kept for its epoch under the envelope's full digest, authenticator included, and never takes the
  command ID, so an envelope sent first on another channel cannot block or answer the solver's press; every other
  answer takes the ID. An envelope whose request digest equals a recorded command's gets that receipt whatever its
  authenticator, and commits nothing. (2) Ruling 1: a press whose preview ended answers `KN5:stale.no_fresh_preview`
  when the slot is empty and `KN5:stale.preview_replaced` when it holds another preview. LI is frozen with the
  architecture, so LI changes follow PHS.52: the amendment adds the caller to LI.5's digest list and changes LI.33's
  sentence to ruling 1. LP3's command-table freeze makes both changes (`layer-interfaces.md`, section 10).
- The owner's readings of 18 November 2026 for the KN5 boundary attack
  (`../docs/wiki/decisions/owner-decisions-2026-11-18.md`; the drafts are in the crate documentation), in the command
  layer:
  - Findings B1 and L17 (KN5.12, ONE.42 option (a)): only the solver's action ends a preview the solver is shown.
    While a frame presents the pending preview, a run's preview and a run's cancel of it are refused with
    `KN5:capability.preview_shown` (a draft code until LP3's freeze registered it), also for the run of a command the
    solver invoked. ADR-D01.9 (owner, 22 November 2026, item 4) replaces the refusal of the run's preview: the run of
    a command the solver invoked may replace the shown preview, the replacement names the preview it replaced, and the
    commit key ignores a press on it for a settle interval from the first frame that shows it
    (`KN5:busy.preview_settling`, a draft; 250 ms until Look Lab tunes it). A run's cancel stays refused, and a
    recording's run never replaces a preview (`LG3:capability.not_granted`).
  - Finding B2 (ONE.34): one check in the layer, not in each host. A run prepares a preview only as the run of a
    program job the host has taken and not yet reported (`ProgramJob::run`, `job-<id>`), invoked by the shell or by a
    run that is itself inside such a command; otherwise `KN5:capability.outside_solver_command` (registered at LP3's
    freeze, like `preview_shown`). The check is on. LP3's host starts every run through a tool row the shell invokes
    (section "Status of LP3, C5 next: the process host and the tool's views"), so the check changes no LP3 route.
  - Finding G1: each run has its own command-ID scope (`runtime/<run>`), so one run cannot use up another run's IDs.
    The scope is derived from the stored caller; the journal format is unchanged.
  - Finding LD1 (EV4.22): a change of the protection policy needs the solver's confirmed press (below).
  - ONE.52: a run invokes no row that writes a workspace record, even a run that a solver command started. The records
    a workspace row's own job returns are written in that row's completion, and only the solver invokes that row.
  - Finding B5 (KN1.11, JI.5; JI.27's alternative is not taken): protection holds inside Jumble, so a round trip through
    Jumble cannot move a protected orbit. A pose move is derived with the jumbling engine's pieces that carry the
    protected orbits as pose-protected pieces (both models read `slot_piece` and `orbit_id` from `assets/model.npz`, so
    the pieces of orbit o are those whose `orbit_id` is o), under the policy in force; a pose move that would move one
    is the engine's `KN2:protection.piece_would_move` and changes nothing. The verdict is the engine's own pose verdict
    (`m_A(w) = 1`), stricter than the label verdict, so no label verdict crosses into Jumble (chapter 03). From solved,
    each item at grips 0 and 1 of the pinned menu moves pieces of all 35 orbits, so while an orbit is protected such a
    pose move commits only after the solver releases the orbit, with a confirmation (LI.26). A reopen derives a stored
    pose move as the live move does, under the protection in force at its place in the journal (finding B7), so one that
    would move a protected piece leaves the layer in read-only recovery before it with the engine's code.
  - Tests: `c600-command/tests/attack_runtime.rs` (B1, B2 and B6/L17),
    `attack_callers.rs` (G1), `attack_playback.rs` (LD1), `strict_policy.rs`, `attack_protection.rs` (B5) and
    `jumble_twist_menu.rs` (B5 under Net and Strict, a pose move that moves no protected piece, and the reopen).
- KN5's stages 3 to 5 (LP2; the drafts are in the crate documentation):
  - The workspace store. `workspace.set` (`RECORD_ROW`, M6's name, which H-08 holds since the command table's freeze,
    CM.17, with `payload: Json` OPEN) writes a named record, its payload and its
    provenance block as one durable action whose receipt is `completed(workspace)`. The workspace revision and the
    record's version advance; the state, the preview slot and the jobs stay (KN5.18, KN5.23). The row takes no token,
    and the runtime may not invoke it. The layer cannot read a provenance block, so the host gives it a `Provenance`
    reader (`CommandLayer::set_provenance`): a block the reader refuses is refused with its error (PS1.16). With no
    reader the row is `KN5:input.row_not_built` and a record draft `LG3:capability.not_granted`; a reopen replays
    record writes without one.
  - `ResolvedDraftRef`: a draft `{"record": name}` previews the letters the record holds once the reader admits the
    record's block in the session (KN5.26). The preview's draft digest covers the record's content digest and version
    (KN0.8), and a record's version advances on every write, undo and redo, so any later write of that record makes the
    preview stale (`KN5:stale.revision_changed`), the same value written again and an undo included; a write of another
    record does not.
  - Workspace undo (`UndoClass::Workspace`): `session.undo` and `session.redo` reverse a record write as a workspace
    inverse, which answers `committed` with the state and the head unchanged and the workspace revision advanced
    (KN5.20, LI.26). The journal keeps the record's value after each write, undo and redo (`Transaction::record`, an
    18th key that only these transactions carry, so a journal without it reads unchanged).
  - Stage 5: `CommandLayer::present(id, &PresentedFrame)` replaces `present(id)`, and `Shell::present` takes the frame
    too. A frame report carries its stamp (device epoch, surface, surface generation, sequence), the preview it shows
    and the chain that drew it (`FrameChain::BuiltIn` or `UserStage`). Reports are read in stamp order, and one that is
    not newer than the newest is refused and changes nothing. A frame that shows the pending preview through the
    built-in chain presents it; a frame that shows no preview, another preview or a user stage's drawing is refused
    with `KN5:stale.no_fresh_preview` and makes a presented preview ready again (ONE.41, RM.26, RPI.76). A word
    commit's token binds the stamp of the frame that presented its preview, and execute refuses with
    `KN5:stale.no_fresh_preview` a token whose frame lies outside the preview's current presentation: one device
    epoch, surface and surface generation, from the first frame that showed it to the newest. `HeadlessFrames` stamps a
    headless adapter's frames for the tests and the LP3 runners.
  - The shell's binding (draft): `Shell::send` drops the preview the shell presented when a committed receipt moved the
    state or a cancel completed. A committed receipt that keeps the state, such as a record write's undo or redo,
    keeps it, as the layer keeps the preview, so a press prepared after it commits.
  - Tests: `c600-command/tests/kn5_stages.rs` (parts of CT-KN5-04, CT-KN5-06, CT-KN5-09, CT-KN5-13, CT-ONE-04 and
    CT-ONE-29), a record write and its undo over the journal in `c600-session/tests/kn5_command.rs` (CT-KN5-04, part),
    and a frame outside the presentation in `kn5.rs`'s token matrix (CT-ONE-03, part).
  - KN5.26's admission matrix (leads L1 and L2 of the KN5 boundary attack): `c600-command/tests/kn5_admission.rs` runs
    it through the layer with `c600-run`'s `boundary::admit_preview` as the host's adapter (a dev-dependency; the layer
    itself never depends on the runtime). A record's word is refused at admission, with no job and nothing moved, when
    its block or any part's block, recursively, is a solver-authored tool's output, research output, a part with no
    stored content or a pick that does not resolve (the last `PS1:capability.provenance_incompatible`, the others
    `LG3:capability.not_granted`). A hand-entered word, an imported 0.4 macro and built-in output pass, and a permitted
    word that moves a protected orbit gets its conflict preview (KN5.9). A reopen keeps every refusal, and a host that
    lost the parts' blocks fails closed. A tool word whose letters the kernel also refuses gets the adapter's refusal,
    never a failed job. A record written between admission and the build never replaces the admitted letters: the
    preview is stale at once. A draft carries no block of the caller's: a draft with any key but `recipe` or `record`
    is `KN5:input.argument_form`, from a run as from the shell (ruling 6's layer half; the runtime's boundary stamps
    and admits a run's own word, `c600-run/tests/boundary.rs`). Parts of CT-KN5-13 and CT-BND-26 to CT-BND-30.
- Strict protection in the command route (after KN5's stages; KN2.4, KN2.7, LB2.3; the drafts are in the crate
  documentation). H-08's `protection.policy.set` (`POLICY_ROW`, `{"policy": "net" | "strict"}`) switches the session's
  protection policy (`CommandLayer::policy`), which is Net from a session's start. The preview, execute's step 8 and the
  undo walk derive under the policy in force: Net judges the complete operation's net map, Strict every executed
  literal prefix (`derive_strict`, `strict_protection`). A preview built under Strict adds `mode: "strict"` and
  `first_refused_prefix` to its summary; a Net summary is unchanged. The row takes no token, and a change in either
  direction needs `confirm: true`, as a release of protection does (`KN5:input.confirmation_required` otherwise;
  finding LD1's reading of 18 November 2026; H-08 gives the row no gate, so the layer asks for it). Its undo and redo
  need none, and the runtime may not invoke it. A change is one durable workspace action: the state and the preview
  slot stay, and the workspace and protection revisions advance, so an open preview goes stale (H-08: "invalidates any
  open preview"). Its undo and redo are workspace inverses. The journal kind is `policy`; the change, its undo and its
  redo carry the policy in force after them as `Transaction::policy`, an 18th key, so a journal without it reads
  unchanged. The stacks are last in, first out, so an undo or redo runs under the policy of its original action. The
  policy lives in the command layer only: M6's own `Session` in `c600-session` has no policy row and judges under Net.
  Tests: `c600-command/tests/strict_policy.rs` (CT-LB2-02's route half, the stale preview, a preview queued before a
  change, undo and redo, a retry across a restart and a reopen, the journal form, a reopen, a commit under Strict
  replayed with its undo and redo, the runtime and the argument form) and a policy change and its undo over the journal
  in `c600-session/tests/kn5_command.rs` (CT-ONE-21, part).
- Stored rows served by a built-in tool's entry (LP3's C6, its first common need; the drafts are in the crate
  documentation). `CommandLayer::serve_row(id, overlay)` lets an entry serve a stored row. An invocation of the row then
  admits a program job for the entry, as for an overlay row, and the entry's result completes it: a query, view or file
  row transiently, a workspace row as one durable workspace action with its result reference. The row keeps H-08's kind,
  lane, undo class, gates and argument types, so a `none`-lane row has no occupancy limit (KN7.2) and a `view`-lane row
  supersedes its own waiting invocation and fences its taken runner (LI.23): the runner is settled as superseded, its
  late result is dropped, and the replacement is taken only after the host reports it. Only a view, workspace, query or
  file row on the `none`, `view` or `program` lane, with no action guard or token, is served, and only with no gate,
  preview rule or undo entry, except a workspace row whose undo class is `workspace`, and one that takes exactly the
  gate `fresh_filter_preview` with the `filter` preview rule; any other is refused with `KN5:input.overlay_row_kind`,
  and a row the layer builds itself or one already served with `KN5:input.overlay_row_conflict`. Serving is not
  journaled, so after a reopen the host serves its rows again. All six of the representations tool's rows and
  `operation.phase.edit` can be served now. Tests: `c600-command/tests/served_rows.rs`.
- Job output (LP3's C6, its second common need; the drafts are in the crate documentation).
  `CommandLayer::finish_program_job_with(job, Result<JobOutput, Error>)` takes a job's output as the runtime's sinks
  gave it: the entry's value, the views it published and the workspace records it wrote, each with its provenance
  block. `finish_program_job` reports a value alone, as before. The result reference carries `JobOutput::digest`, which
  is the value's own digest when there are no views and no records. Only a workspace row's job writes records; a record
  from any other row fails the job with `LG3:capability.not_granted`. The host's provenance reader checks every
  record's block, as for `workspace.set`. A job whose output fails a check writes nothing. A workspace row's records are
  written in its completion's one durable action and kept in the journal (`Transaction::records`). For a row whose
  undo class is `workspace`, every language-defined workspace row (LG3.5, LB7.2; LP2's reading 18) and a stored row
  such as `filter.apply` or `operation.phase.edit`, the completion is a workspace inverse: its undo writes back what
  each record held before, absent ones included, and its redo writes the records again. No undo reaches the records
  of a stored row whose undo class is `none`, so earlier writes of the same records leave the undo and redo stacks,
  and so, beneath an action that leaves, do the actions that wrote one of its other records. A replay decides from the
  table it opens with, since rows are served and registered after the open: a stored row's undo class is H-08's, and
  a row the table does not hold is a language-defined one. Tests: `c600-command/tests/job_output.rs` and
  `c600-command/tests/filter_apply.rs`.
- Saved sets (LP3's C6, `set.save`; U1-057, U4-037; LP2's pull request 337; the drafts are in the crate
  documentation). `set.save`'s entry writes the one record `set.save`, `{kind, members, name}`, and in the
  completion's one durable action the layer writes the saved set's own record `set/<name>` in its place
  (`c600_command::sets`): the kind, the members ascending and distinct, the identity of the model the job read, the
  name and the capture revision. The result reference carries the exact set's digest, the SHA-256 of the FP.5 bytes of
  `{kind, members, model}`, so saves of the same members of one kind under one model share a digest whatever their
  names. Names under `set/` are reserved: only a `set.save` completion writes them, and `workspace.set` and
  `worksheet.save` refuse them with `KN5:input.argument_form`. A save under a name that holds a saved set is refused
  with `KN5:input.saved_set_exists` (a draft code until LP3's freeze registered it), at admission with no job and at
  completion, writing nothing, when another save took the name meanwhile, unless the invocation carries the
  boolean `replace: true` (H-08's `replace: Bool?` since the freeze, CM.17). Tests:
  `c600-command/tests/saved_sets.rs`.
- Argument blocks (BND.22; LP3's C6, step 1 item 4; the drafts are in the crate documentation).
  `CommandLayer::invoke_with_inputs(caller, invocation, inputs)` takes the provenance blocks of an invocation's
  arguments beside the envelope. The host builds them; for a running program's invocation it builds them from the run's
  context. Only a row an entry serves takes blocks (`KN5:input.argument_form` otherwise). The host's reader checks each
  block after the arguments' forms and the gates, and without a reader such an invocation is refused with
  `KN5:input.row_not_built`. The job holds the blocks as given (`ProgramJob::inputs`), and the host runs the entry with
  them as its run's inputs. The request digest covers each block's SHA-256 only when there are blocks, so `invoke` keeps
  every earlier digest. A run still reaches no workspace or file row. Tests: `c600-command/tests/job_inputs.rs`.
- Filter previews and `filter.apply` (KN0.9, H15.4, LI.41, CT-LI-11; LP3's C6; the drafts are in the crate
  documentation and `c600-command/src/filter.rs`). A `FilterRef` is the result reference `{"job", "digest"}` of a
  completed `structure.layer` job, and it resolves while every record that job wrote keeps the version the write left
  (`KN2:input.named_object_invalid` otherwise). A completed `filter.preview` job is kept as a filter preview
  (`CommandLayer::filter_preview(job)`), bound to `FilterPreviewDeps`: the epoch, the model identity, the mode, the
  state revision and the digest of the resolved filter, which holds each record it read with its content digest and
  version. The protection and workspace revisions are not dependencies, so an unread edit leaves it fresh. A
  `FilterPreviewRef` is `{"epoch", "job"}` (`FilterPreview::reference`): job IDs start again after a reopen, and the
  epoch keeps a reference from before it off a later preview. The gate `fresh_filter_preview` reads neither the
  word-preview slot nor a
  presentation: a missing or stale filter preview refuses `filter.apply` with `KN5:stale.filter_preview` before any
  job, and the gate is checked again when the job's output is published, where a stale one fails the job and writes
  nothing. Filter previews are transient, so a reopen keeps none. Tests: `c600-command/tests/filter_apply.rs`.
- Argument forms (LP3's C6, its third common need; the drafts are in the crate documentation). An argument of an
  optional type (`T?`) may be left out of any row; every other argument must be given. A row the layer does not build
  is refused as not built once its names fit, with an optional argument left out or given. For a row the layer admits
  as a program job, a served row or an overlay row, it checks the forms of `Nat`, `Text`, a choice of words, `CellId`
  (0 to 599), `CellIdPair` and `RevisionRef` (KN0.2's nine keys) before it admits the job. A malformed argument is
  refused with `KN5:input.argument_form`, and after every form check a `RevisionRef` that is not the current revision
  in every field is refused with `KN5:stale.revision_changed` (EV4.14). Both refusals come before the lane's capacity
  and take no job. `FilterRef` and `FilterPreviewRef` have their forms too (the filter previews item above), and what
  they name is checked after every form. `CanonicalObjectRefs` (`set.save`'s members) is a list of pieces or positions,
  each 0 to 177,119, the kernel's `PIECES`; an empty list and repeats pass (draft, after LP3's `save_set`), and
  `set.save`'s `replace` is a boolean (H-08's `replace: Bool?` since the freeze, CM.17; saved sets, above). Other types
  pass unchecked until their forms are written down. Tests: `c600-command/tests/argument_forms.rs`.
- Overlay availability (KN5.5, KN5.7, LI.18 to LI.21; the drafts are in the crate documentation), computed with no
  language code running (CT-KN5-08). `CommandLayer::declare_overlay(id, Metadata)` stores a language-defined row's
  metadata: the static check's first failed diagnostic, and the kernel rows its program may invoke, each required or
  optional and bound `Binding::Static` with captured arguments or `Binding::Computed`. `availability` answers such a
  row from that data and the layer's state, in LI.20's order: the diagnostic; a required row the runtime may not invoke
  (`LG3:capability.not_granted`); the row's own gates (read-only recovery for a row that changes the session, then a
  full program lane); the first failing required row in sorted row ID order, ties in stored order. A required row that
  is not a kernel row is `KN5:input.overlay_dependency_missing`. A static binding answers as
  `availability_for(row, args)`, LI.18's argument-aware hint, does; a computed one by the row's argument-independent
  gates. Optional rows take no part, and a hint starts no program and takes no lane. A registration, a served row or a
  declaration advances the generation (LI.21). Metadata is not journaled, so after a reopen the host declares it again.
  `CommandLayer::availability_table(bindings)` is KN5.6's draft table over LI.18's hints (LP2's pull request 377): one
  answer per binding in an `Availability`, keyed by the row and the digest of its absolute arguments (none for computed
  ones, LI.19), with the revision, the generation and a digest of the caller's context read at one moment; it grants
  nothing either. Tests: `c600-command/tests/availability.rs`, which also reads the crate's dependency closure from the
  manifests: the kernel's crates and no language crate. A full read after missed changes equals the last of the
  stepwise reads (CT-LI-07 in part).
- The report after a command (KN5.22, R-02, ONE.70; the drafts are in the crate documentation). `report` pairs a
  receipt with the display's status, and `Shell::send_reported` sends and reports. When the command applied
  (committed, or completed as a workspace write, a view publication or a file publication), the display refreshes to
  the head through the host's `Refresh`; a failed refresh reports "Applied, display out of date"
  (`DISPLAY_OUT_OF_DATE`) and leaves the receipt, the outcome lookup, the preview and the jobs as after any commit. The
  continuation, `refresh_display`, sends no command. The host's `Refresh` over LP3's view model and the message on
  screen are LP3's (CT-KN5-05 in part). Tests: `c600-command/tests/refresh.rs`, a port of 0.4's
  `tests/test_native_reply_after_commit.py`, and a restart that keeps the durable terminal outcomes and answers the
  transient ones `unknown_outcome` (CT-LI-02 in part).
- LB7.2's work sheets (H-08's `worksheet.save`, `worksheet.load` and `worksheet.compare`; solving workflow section 4;
  the drafts are in the crate documentation). A work sheet is a workspace record whose payload is `{"worksheet":
  {...}}` (`Worksheet`): the phases Prepare, Macro and Cleanup, the named roles (a piece, by its home position, or a
  position), the reference as given, the piece roles' placements and the position roles' occupants (`Resolution`,
  `Occupancy`). A draft `{"record": name}` of a sheet previews its complete operation (`RecordValue::operation`).
  `worksheet.save` takes a draft `{"record": r}` whose record holds letters or a sheet, and a `name`; the sheet keeps
  that record's provenance block, which the reader checks again, its piece roles are placed at the head through
  KN2.4's `positions` row, and each position role's occupant is read there from KN2.4's `labels` row: the piece whose
  stickers fill all of the position's slots, or none when no one piece does (LP2's pull request 361). A sheet saved
  from a record of letters keeps the record's name, version and content digest as its macro version (`MacroVersion`).
  `worksheet.load` takes H-08's `WorksheetRevisionRef` as `{"record": name, "version": n}` (`WorksheetRef`); another
  version is `KN5:stale.revision_changed`. It places the piece roles and reads the occupants again, keeps where each
  piece was saved and which piece filled each position then (`Placement::moved` and `Occupancy::changed` show a
  mismatch, and nothing is retargeted) and writes the sheet again, so every preview that read the sheet goes stale,
  also at the same state, and the reuse needs a fresh engine preview. Each of the two rows is one record write,
  undoable as a workspace inverse (LB7.2; H-08 gives `worksheet.save` `undo: none`). `worksheet.compare` takes the
  `WorksheetRevisionRef` and a draft `{"record": r}`, the current work, and runs as a job on the exclusive lane in the
  layer (`CommandLayer::run_jobs`), since no entry can serve that lane: at the head it compares the sheet's inputs
  with the draft's (`compare_inputs`), places the piece roles again, names the position roles whose occupant changed
  (`occupant_changed`), reports the macro version as current, changed or missing, and analyses the sheet's complete
  operation afresh with LB2's gains and losses. The comparison is the completion's result
  (`CommandLayer::comparison`); it grants nothing and writes no record. The three rows need the host's provenance
  reader and Jumble off; the runtime may invoke none, and no entry can serve them. Tests:
  `c600-command/tests/worksheet.rs` (CT-LB2-03's work-sheet part), the save and load rows with Jumble on in
  `jumble_on.rs`, and the three rows on M6's journal in `c600-durable` (C7's section below).
- `twist.apply` (A6; ONE.46, ONE.78, KN3.7 to KN3.9; the drafts are in the crate documentation), a live twist
  committed at once with no preview. The host pins the twist menu with its exact model (`CommandLayer::pin_menu`);
  without a pin the row is refused with `KN5:input.row_not_built`, and the pin is not journaled. The row's `cap` and
  `rotation` (KN3.7's binding without its cap: exactly `request`, `operation` and `menu`) form the exact request, which
  resolves against the pinned menu. A malformed binding is `KN2:input.rotation_request_malformed`, a free binding or
  another menu pin `KN2:input.menu_binding_mismatch`, a direct rotation off the menu `KN2:input.twist_off_menu`, a
  jumble item `KN5:input.menu_word_needs_jumble_on` and the identity item `KN2:input.recipe_shape`, all before the
  token, so they spend no press. A retained item commits its shortest retained word as a one-word recipe under the
  protection policy in force. The token binds the state and protection revisions (LI.6), so a press from before a
  commit, a protection change or a policy change is `KN5:stale.revision_changed`. Like a word commit, the twist undoes
  and redoes, ends the pending preview and cancels the jobs that read the replaced revision, except file jobs (LI.32).
  The journal keeps the resolution's record as `rotation` (`Transaction::rotation`), an 18th key on a twist only, and a
  reopen replays the recipe without a menu (KN3.9). Tests: `c600-command/tests/twist_apply.rs` (the command layer's
  parts of CT-EV4-04 and CT-BND-19; KN3.8, JI.35, LI.6, LI.32).
- `jumble.on` (A5; ONE.54, ONE.55, ONE.80, ONE.81, JI.25 to JI.27, JI.29, FP.31, FP.32; the drafts are in the crate
  documentation), the entry into Jumble on from an active exit's root or the solved witness. It needs the pinned twist
  menu (`KN5:input.row_not_built` without one) and takes `revision`, the current `RevisionRef`, with a token that binds
  the state and protection revisions. An exit is active while the path from the head climbs through retained commits
  only to it (JI.26; no void is built). The entry then replays that retained suffix from the exit's own poses in the
  jumbling engine under the pinned menu (FP.32), and an empty suffix (FP.31) keeps the exit's poses. Poses made under a
  menu with another identity block are `KN1:input.identity_mismatch`, and the solved witness is not tried while an exit
  is active. With no active exit the witness is the chronological literal retained path from the session's start to
  the head, which must start at solved, replayed from solved. The entry enters only when the replay's labels are the
  current labels, so labels alone, or a suffix that does not replay, are `KN5:input.jumble_on_labels_only`. At the
  layer's bound of 64 chained derivations an entry replays the jumbling journal flat from solved instead. The crossing
  commits with no preview: the revision's mode becomes `jumble_on`, its state revision advances and its digest is the
  pose digest (`CommandLayer::jumbled` holds the poses), and like a commit it ends the pending preview and cancels the
  jobs that read the replaced revision, except file jobs. With Jumble on, `operation.preview`, `operation.execute`,
  `twist.apply` and `jumble.on` itself are `KN5:input.retained_tool_needs_jumble_off`, while the protection, policy,
  record and program rows stay. An undo of the entry returns to the exact Jumble off state before it (JI.29), and a
  redo makes the same entry under a menu with the same identity block (`KN1:input.identity_mismatch` otherwise), which
  must reach the entry's recorded poses and record. The journal keeps the crossing as the action kind `crossing` with
  its record (`Transaction::crossing`), an 18th key on a crossing only: the entrance, the identity block, and the
  witness's letter count and `Digest<literal word>`; from an exit root also the exit's event (`crossing`) and the
  SHA-256 of its canonical record (`exit`), with the suffix as the witness. A reopen with the menu pinned
  (`CommandLayer::open_with_menu`) replays the entry and must make the same record; without it the layer stays in
  read-only recovery before the crossing. A crossing under a store fault commits whole or not at all (CT-LI-05 in
  part). Tests: `c600-command/tests/jumble_on.rs` and, for the entry from an exit
  root, `c600-command/tests/jumble_off.rs` (the command layer's parts of CT-ONE-03, CT-ONE-11, CT-ONE-17 and
  CT-JI-08).
- `jumble.twist.menu` (JI.27, JI.35, FP.31, ONE.78, KN3.7 to KN3.9; the drafts are in the crate documentation), a live
  twist of an exact item of the pinned twist menu, committed at once with no preview. It needs the pinned menu
  (`KN5:input.row_not_built` without one) and takes `cap`, `letter` (KN3.7's binding without its cap, the form of
  `twist.apply`'s `rotation`) and `revision`, the current `RevisionRef`, with a token that binds the state and
  protection revisions. The binding resolves as for `twist.apply`; its refusals, the identity item's
  `KN2:input.recipe_shape` in both modes and, with Jumble off, a jumble item's `KN5:input.menu_word_needs_jumble_on`
  come before the token. The named revision must be current in every field (EV4.14), so unlike a `twist.apply` press a
  press from before a workspace completion is `KN5:stale.revision_changed`. With Jumble off a retained item commits its
  retained word as `twist.apply` does, under the protection policy in force. With Jumble on every item is a pose move:
  the jumbling engine derives the item's exact twist from the head's poses under the policy in force, with the pieces
  that carry the protected orbits as pose-protected pieces (finding B5, above), so a twist the poses block is
  `KN2:input.word_blocked_prefix` and one that would move a protected piece `KN2:protection.piece_would_move`; the
  revision's digest becomes the new pose digest and its state revision advances. Like a commit it ends the pending
  preview and cancels the jobs that read the replaced revision, except file jobs. An undo replays the twist's exact
  inverse and a redo the twist, each checked
  against the recorded poses; after 64 chained moves the layer replays the jumbling journal flat from solved and checks
  it against the chained poses. The journal keeps a pose move as the action kind `jumble_move` (FP.31) with its rotation
  record (`Transaction::rotation`), and a reopen with the menu pinned (`CommandLayer::open_with_menu`) replays it; a
  jumble move under another row, or a retained commit while Jumble is on, leaves the layer in read-only recovery. Tests:
  `c600-command/tests/jumble_twist_menu.rs` (the command layer's parts of CT-EV4-04, CT-ONE-03, CT-ONE-11, CT-JI-06 and
  CT-KN5-04).
- `jumble.off` (A5; ONE.54, ONE.80, ONE.81, JI.27, FP.31; the drafts are in the crate documentation), the exit from
  Jumble on at A5's label checkpoint. It needs the pinned twist menu (`KN5:input.row_not_built` without one) and takes
  `revision`, the current `RevisionRef`, with a token that binds the state and protection revisions; with Jumble off it
  is `KN5:input.menu_word_needs_jumble_on`. The named revision must be current in every field (EV4.14). The jumbling
  engine's checkpoint (`levels::label_checkpoint`) judges the head's poses along the jumbling journal from solved, and
  ν = 0 shows from the identity labels, a retained witness or ν as computed (`nu::nu`). Pieces off the lattice are
  `KN5:input.jumble_off_pieces_off_lattice`, a nonzero ν `KN5:input.jumble_off_nu_nonzero`, and poses not shown
  reachable (no witnessed chain, or a displaced centre) `KN5:input.jumble_off_not_shown_reachable`; each names counts
  only and changes nothing. An exit commits with no preview and no protection gate (no pose move moved a protected
  orbit's pieces while it was protected, finding B5): the labels' state of 600-cell-Full
  becomes the retained state, the revision's mode becomes `jumble_off`, its state revision advances and its digest is
  the label digest. Like a commit it cancels the jobs that read the replaced revision, except file jobs. While only
  retained commits follow the exit it stays active, and `jumble.on` enters again from its poses (JI.26). An undo
  restores the exact poses before the exit and the entry's retained state; a redo runs the checkpoint again and must
  reach the same labels and record. The journal keeps the exit as the action kind `exit` (FP.31) with its record under
  `crossing`, an 18th key: the episode, the jumbling identity block, the journal's letter count and word digest, the
  pose and label digests and A5's certificate. A reopen with the menu pinned replays the checkpoint and compares the
  record; a record or row that differs leaves the layer in read-only recovery at Jumble on. Tests:
  `c600-command/tests/jumble_off.rs` (the command layer's parts of CT-EV4-04, CT-ONE-03, CT-ONE-11, CT-ONE-17,
  CT-JI-01, CT-JI-09 and CT-KN5-04).
- The jumbling oracle route `c600-jumble-route` (LP3 plan, PHS.23's "Oracle equality at every committed step", its
  jumbling half: M5's oracle through the command layer with Jumble on, C8) serves M5's adapter protocol: `start` pins
  the menu and presses `jumble.on`, each `apply` presses `jumble.twist.menu` with the item the harness names at the
  current revision, and `undo` and `redo` press `session.undo` and `session.redo` back to the crossing at most.
  `tests/test_jumbling_command_route.py` builds it in release and compares it with J1 through the differential harness
  step by step on the first 70 records of each W-J fixture (`JUMBLING_FULL_REPLAY=1` runs every record): the status, the
  moved count, the moved set, the pose digest and the lattice digest. Then every move undoes to the entry and redoes to
  the end onto J1's states of its prefix, moving its own pieces (JI.27); seventy moves pass the layer's bound of 64
  chained pose moves, so the moves, the undos and the redos each cross a replay from solved. A blocked item, a record
  off the menu and a malformed record change nothing. Not compared: the identity item, which the command layer refuses
  with `KN2:input.recipe_shape` where J1 moves its inside pieces onto their own poses (a lead for LP3), and free mode, a
  budget and protected pieces, which the route answers with input errors. `tools/ci/run_headless.py` lists it. C8's
  `c600-jumble-tool/tests/jumble_on_layer.rs` presses the same layer in process beside M5's engine, every record of the
  whole journals on request (C8's section below).
- EV3.2's view rows `view.open` and `view.close` (RPI.81 to RPI.83; the drafts are in `c600-command/src/view.rs`). The
  layer admits them, and the host's view model holds the views, Global and Local and RPI.82's budget: the `Views`
  trait, given with `CommandLayer::set_views`, which LP3's host implements with `c600-view`'s `ViewModel`. Without one
  both rows are `KN5:input.row_not_built`, as before. Before the host is called: the argument forms (a `ViewId` is a
  nonempty text; a `ViewSourceRef` is `{"built_in": Text}` or `{"overlay": {"row", "module", "arguments"}}`, the module
  a SHA-256 in lowercase hex), an overlay source's row, which must be an overlay row of kind `view` with arguments that
  fit it (`KN5:input.argument_form`), and the caller: a run not inside a command the solver invoked, as a
  recomputation started by a revision change is not, is `LG3:capability.not_granted`, so a view cannot open views. The
  host's refusals (`EV3:input.view_id_in_use`, `EV3:input.view_not_closable`, `EV3:resource_limit.display_plan_ceiling`)
  reach the receipt. An admitted open or close is `accepted` with no job, since KN5.21 has no completion kind for either
  row; a retry gets the stored receipt and the host sees one call. Tests: `c600-command/tests/view_rows.rs` (CT-RPI-36's
  layer half and CT-EV3-07's view half on the layer). LP3's host, `c600_tools::views::ModelViews`, serves the rows with
  the view model in `c600-tools`' `tests/view_host.rs` and `c600-durable`'s `tests/kn5_on_layer.rs` (C5's and C7's
  sections below).
- The session's durable home for a word's part blocks (BND.24, BND.85, CT-BND-27; draft). The host's reader has two
  more methods, with defaults that keep a reader without them working as before: `Provenance::parts(block)`, the
  blocks of the block's recursive parts that the host holds, and `Provenance::admit_kept(block, kept)`, admission with
  the part blocks the session keeps. A record write (`workspace.set` and the work-sheet rows) and a workspace row's
  completion keep the part blocks the reader holds for the blocks they write, each by the SHA-256 of its bytes. The
  action journals those the session did not keep yet as `blocks`, a draft field of the journal form that only a record
  write or a workspace completion carries. A replay keeps them again, and nothing removes them, an undo included;
  `CommandLayer::blocks` lists them. At `operation.preview` of a record draft the layer gives them back to the reader,
  so a restored word's recursive parts are read after a restart with a reader that holds nothing. Tests:
  `c600-command/tests/part_blocks.rs` and `c600-session/tests/part_blocks.rs` (CT-BND-27 on the journal after a power
  loss, with `c600-run`'s FP.17 blocks and the boundary's preview admission as the reader).
- The session's profile (BND.14 to BND.17, KN5.26; LP2's pull request 287; draft): `solving`, `tool_authoring` or
  `research`. The host sets it when it creates the session (`CommandLayer::new_with_profile`; `CommandLayer::new` is
  `solving`), the session's start records it once (`Genesis::profile`) and a reopen reads it back; no program or row
  changes it. KN5.26's admission of a record's word runs only in a solving session, and in `tool_authoring` and
  `research` a run may not invoke `operation.preview` (`LG3:capability.not_granted`). Tests:
  `c600-command/tests/profile.rs`, which also invokes every row of the table on a run's channel in each profile: none
  commits, the 19 commit and app rows are refused `COMMIT_ROW_REFUSED`, and `jumble.scramble`, the one other row
  that takes a token, `TOKEN_INVALID` (CT-ONE-02 and CT-KN5-02, with `c600-run`'s and `c600-session`'s halves). LP3's
  hosts start runs in solving sessions only; CT-RTE-07's B13 and B14 refuse a
  command in a tool-authoring session before any run (C5's section below).
- Budgets (KN7.7, BM3, DET.27, ERR.27; LP2's pull request 348; draft). An invocation of a row an entry serves declares
  its budget in the envelope (`Invocation::budget`), and the shell declares its own (`Shell::budget`, by default
  `Budget::CEILING`) on those rows only. The layer admits it before a job ID exists, so a refusal uses nothing: a missing
  budget is `KN7:input.budget_not_declared`, and a dimension over the ceiling `KN7:resource_limit.budget_ceiling`. The
  ceiling is the profile's (`Budget::CEILING` for every profile: steps 2^21, kernel cost 2^40, memory 2^27 and output
  2^24, owner, 21 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-21-ceilings-e3.md`),
  or the host's where the host sets a lower one (`CommandLayer::set_budget_ceiling`), dimension by dimension. The job
  carries the admitted budget (`ProgramJob::budget`), and the request digest covers it. Tests:
  `c600-command/tests/budget.rs`. LP3's hosts run a job's entry under the run limits of its budget, and B13 and B14 also
  run on the 600-cell-Full tool's row (C5's section below).
- Leads L12, L13 and L16 of the KN5 boundary attack (LP2's pull request 369) and the unavailable presentation outcome
  (pull request 371), in the command layer:
  - L12 (CT-KN5-05, part): an action the store wrote before a panic unwound out of the write is held with read-only
    recovery (`KN5:unknown_outcome.commit_fate`) and settled by `restart` from `Store::durable`, as after an unknown
    fate (`c600-command/tests/commit_fault.rs`).
  - L13 (b) and (c): a program job's terminal and a device loss's un-present each advance the generation
    (`tests/layer.rs`). L13 (a) is structural: the layer's mutators take `&mut self`. L16 (a): `tests/worksheet.rs`.
    L14 apart from (e), and L16 (b) and (c), were built earlier; L17 is the item "Findings B1 and L17" above.
  - The unavailable presentation outcome (RM.26, LI.21). When the adapter cannot confirm presentation, for example
    after a device loss, `CommandLayer::presentation_unavailable` clears the presented-preview latch and never guesses
    the newest preview it received. `Shell::presentation_unavailable` reports it and ends the shell's preview binding,
    so no token names the preview meanwhile (`tests/layer.rs`).
- KN5 not built yet: BND.16's cells that grant a run more outside a solving session (the authoring workspace's records
  and research records); ruling 6's `file` overlay without a granted file capability (L14 (e), which waits for an owner
  reading); L13 (d), a draft KN5.6 table object; the typed workspace rows and limits on a record's payload; a display
  adapter's own frame reports, for which `HeadlessFrames` stands in. No host caller is built: callers are stamped on
  two channels, the shell's and LP3's runtime bridge (`c600-tools`). `JournalStore` takes M6's session lock, so a
  second opener of a held session is refused with `KN4:busy.session_locked`.
- The integrator's fresh Claude read of KN5 found one major problem and it is fixed: a restart that could not read
  the journal back treated an unknown fate as absent. `Store::durable` now answers `None` when the store cannot tell.
  The transaction then stays unknown and the layer stays in read-only recovery, so no later action takes its order.
  One minor problem is fixed as well: a runtime envelope without the token its row takes no longer authenticates. Open,
  deferred as minor:
  - A retained refusal's receipt carries the envelope's full digest, where KN5.21's receipt has the request digest.
    So `outcome` given the request digest alone answers `unknown_outcome`.
  - After a reopen, `invoke` refuses an old-epoch envelope whose command ID was never durable with
    `KN2:stale.epoch_changed`, and that refusal takes the ID. `kernel_outcome` answers `unknown_outcome` for the same
    ID.
  - The caller label is the host's to set, and `protection.orbit.set` takes no token. Until stage 3's admission matrix,
    a release's `confirm` is not authenticated as the solver's.
  - Retained refusals are not bounded within an epoch.
  - The key route cannot send `confirm`, so undoing a protection add by key answers `KN5:input.confirmation_required`.
- Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg was not run. The timings the loop
  check prints are informational, not performance evidence.

## Status of LP3, C2: display and views

- `c600-view` is the host side of chapter 10's view interfaces without a device: which views are open, which engine
  revision they show, which program output may attach to it, what the view lane runs next and what the views cost.
  Global and Local are always open and visible; further views open and close through `view.open` and `view.close`
  (RPI.81 to RPI.83). A complete engine bundle is adopted whole or not at all (RPI.9, ONE.60, ONE.61); output bound to
  an older revision shows as stale and is never drawn over a revision it does not match (ONE.12, ONE.63, RPI.13). The
  view lane keeps one runner and one newest pending request (KN7.2, LI.22 to LI.25, ONE.64). Hidden views do no work
  (RPI.11, RM.25), and the view budget charges RM.11's limits, RM.13's output allowance and RM.38's frame work.
- Tests (`c600-view/tests/view_rows.rs`): CT-RPI-03, CT-RPI-06, CT-RPI-07, CT-RPI-36, CT-RPI-37, CT-RM-02 and
  CT-ONE-30; the limit half of CT-RPI-22; rows 1, 2, 7 and 9 of CT-RM-09's RM.40 table. Supporting tests cover ONE.10's
  reuse after undo, RM.11's profile validation, RM.12's admission ceiling, RM.35's malformed output and RPI.9's bundle
  faults. C2's next view piece adds CT-RPI-23 (Global and Local stay linked and further views open under every
  profile fixture; built-in output keeps the interim boundary under either chart evaluator, with a declared fixture
  chain standing in for a checked vertex stage), the counter half of CT-RPI-26 (hidden, evicted and unchanged views
  add no rebuild; nothing is timed) and the admission semantics of CT-EV3-08 under fixture ceilings, and three
  supporting tests: a burst of revisions while the lane is blocked leaves one pending request at the newest; output
  from an older engine epoch never attaches at the same revision numbers; a scheduled recomputation can neither open
  nor close a view.
- Drafts, which freeze with the IR or stay open past H-08's freeze: the list in the crate documentation, among them
  Global and Local's fixed IDs, `LG3:capability.not_granted` for a recomputation that names a view row (RPI.83 names no
  code), a separate command-ID namespace for scheduled recomputations (LI.25), the status precedence (RPI.13, charter
  D2), and an LEB128 byte form standing in for H8's canonical output bytes. C2's next view piece adds one view model per
  session profile, which no rule reads yet (LG3.9, RPI.42); a provenance block the host fills for installed output
  (RPI.45, BND.22); the built-in class declared when a view opens (BND.27); the chart evaluator as the view's declared
  chain (RPI.28, RPI.68); rebuild counters per view; a command ID on `view.open`, with scheduled IDs refused in both
  view rows (RPI.83, LI.25); and engine epochs in revisions, bundle digests and the publication bytes (now version 2): a
  bundle from an older epoch is refused with `refresh_required`, and output attaches only within its own epoch
  (KN0.2, ONE.60, ONE.63).
- Source edits (for C9's LX-4): `ViewModel::edit_source(version, draft)` takes the editor's draft of a module version,
  both source digests. Installed output whose producer block names that version is `stale` from that call on, with a
  `SourceLag` (the producing version and the draft) in `ViewStatus::Stale` and in its `FrameView`; returning the draft
  to the version's bytes clears it. It is a host comparison of digests (RPI.13), not a revision part (ONE.8) or a
  workspace change (KN0.2): nothing runs and nothing is fenced, since an unchecked draft never runs, even through a
  view (EV1.7), and the output is still drawn while its base matches, its picks opening the producing version (EV3.5,
  EV3.6). ONE.12's table names the view-stale condition for a lagging revision only; a lagging source is the
  integrator's reading of EV3.6 and RTE.49's stale-state interval, and the two show together. A view shows a checked
  draft's output once it is reopened on it after activation (EV1.11 is LP4's). Drafts: one draft per version, kept
  until the editor returns it. Tests: `ev1_07_` and the `ev3_06_` tests in `view_rows.rs`. Nothing is timed; LX-4
  times the state on the owner's machine (C9).
- Not built in the views: CT-RPI-26's gate half (charter E2 and the R-19 captures) and CT-EV3-08's measured half
  (RM.39's measured ceilings) wait for C9; LQ2's policy table and display refusal wait for the joint setting.
- `c600-display` is the host side of chapter 10's display plans and RM's feature matrix without a renderer. The
  validator checks RPI.10, RM.11, RM.12 and RM.20 in the order invalid, unsupported, ceiling. A publication installs
  whole or not at all and ends in exactly one KN5.21-style terminal, standalone or scheduled (RM.17 to RM.26, LI.25).
  After a device recovery the host prepares a retained publication again under its own command and generations, and the
  view draws it only once the new resources are active (RM.23). A mock backend stands in for an adapter (RPI.55, RM.33).
  Picks resolve by instance ID through the installed source map, never through a chart inverse, and stale picks name the
  field that changed (RM.10, RM.24, RPI.24, RPI.25). Each slot projection holds one level: a subject at the other level
  gets no slot, with reason `KN1:input.level_not_held`, before any digest is compared (KN1.8, KN1.10).
- The `Backend` trait carries RM.14's other adapter operations (C9's trait gap, option (a)): the host validates an
  engine bundle and adopts it through the backend, also while the device is lost, and a bundle either side refuses
  changes nothing (RM.16); a frame report marks the fallback of the installed publication it names, changes no receipt,
  and is dropped when it is of an earlier device epoch (RM.41); a pick is requested for a frame the backend was given,
  polled, and its stamped instance resolved as above. A pick is stale when its frame is of an earlier device epoch or
  the device was lost after it was asked (both at `device_epoch`), and when the backend refuses or answers it as stale,
  a frame it no longer keeps included: an explicit pick then returns `EV3:stale.display_pick`, and a passive one is
  dropped, with no ticket when the request itself is refused (RPI.25). Tests: `adapter_ops.rs`, over the mock backend.
- Charts and stages (C2's next display piece): exact arithmetic in Q(√5) with checked `i128` parts, the 600-cell and its
  exact poses in K⁺ (`exact.rs`); chart checks with the exact region test, the vertex test, and injectivity, embedding,
  isometry and homomorphism each decided by its own check on `c600_jumble::polytope` (C6), and `admit_chart`, which
  admits a chart only on established check results and refuses it on a refutation with `EV3:evidence.chart_check_failed`
  (`chart.rs`); a binary32 CPU reference evaluator and RPI.56's key over the exact fields (`reference.rs`); and the
  stage output rules and their composition order (`stage.rs`). C2's stage take-up reads a stage's description from a
  stage that LP2's checker admits (`c600_check::admit_stage`, pull request 241) and returns each refusal with the
  checker's code (`stage::take_up`); it adds RPI.72's raster bound on the CPU over placed triangles (`raster.rs`) and
  the frame stop, the session's quarantine and RM.41's frame reports beside the host (`stage::Quarantine`).
- The relationship strip's data contract (`strip.rs`, C6): EV3.8's fields of one typed relation, which EN3's view model
  carries without deciding the strip's design: its statement key, the relation, the roles of its operands and values
  (with "frame" qualified by its use), its identity block, level and base revision, a result per operand, evidence
  references and a source location. "None: off the lattice" and `unknown(reason)` are two values (EV3.9), and a
  result is total, partial or undefined (KN0.4). `strip::attach` shows a record only on a relation node of its base
  revision whose claims carry its statement key (RPI.65), and refuses with `EV3:input.display_plan_invalid` a record
  that read another model, one that lifts labels into poses (KN1.10) and one that answers "none" from labels. Nothing
  here computes a relation: a view's run does (EV3.10). Its tests are C6's, in `c600-repr/tests/ct_ev3.rs`.
- Display tests (`c600-display/tests/`): CT-RPI-02, CT-RPI-04, CT-RPI-05, CT-RPI-09, CT-RPI-10, CT-RPI-11,
  CT-RPI-27, CT-RM-03 and CT-RM-04; CT-RPI-08's 600-cell-Full rows (F01 to F17, with J02 and J04 unsupported on
  Full); the stale half of CT-RPI-12; RM.37's fifteen rows of CT-RM-07 as fixture payloads over the Full geometry
  digests. C2's next display piece adds CT-RPI-14 over the drafted chart repertoire (RPI.34 is open), CT-RPI-18 on
  the CPU reference (its device version, CT-RPI-33, waits for the renderer), CT-RPI-19's Windows leg, CT-RPI-22's
  chart half (with the views' limit half, CT-RPI-22 runs whole) and the headless part of CT-RPI-32. C6 adds CT-LB4-01,
  whole over the drafted repertoire (`ct_lb4_01.rs`): the domain with its boundary cases, and injectivity, embedding,
  isometry and homomorphism each decided by its own check, for stereographic and projective charts. C2's stage take-up
  runs CT-RPI-32's headless part on stages that LP2's checker admits, each refusal with the checker's code, and the
  headless parts of CT-RM-09's rows 3 to 6, 8 and 10 (`ct_rm_09.rs`): spread and overlapping plans, samples and the
  ID pass, a near-plane crossing, a line-only layout, a later-frame stop that keeps its receipt, and a chain's stages
  quarantined together. RM.23's recovery runs in `publication.rs`: a retained publication is prepared again on the new
  device before it draws, waits while its view has pending work, gives way to a newer publication, and is not prepared
  again when it is stale or its view was closed; a preparation that is refused, cancelled or lost with the device leaves
  it unavailable until a new publication or the next recovery.
- Display drafts: the list in the crate documentation, among them the validation order, string dependency keys, set
  digests over sorted piece IDs, canonical JSON standing in for the output bytes, form `stop` for every error after
  acceptance, and a publication to a view that is not open being invalid. C2's next display piece adds the drafted chart
  repertoire (stereographic charts declared by their pole, over convex hulls of exact vertices, with the properties
  domain, injective and section; RPI.34), which C6 extends with projective charts x ↦ (M·x + t)/(c·x + d), regions that
  are unions of cells, and the properties embedding, isometry and homomorphism; exact poses only (every float pose is
  refused, never rounded; RPI.30, LB4.2), Q(√5) arithmetic local to the crate (KN3.2, KN3.3), the fixtures' own
  precision rule (1e-4 absolute, with an exclusion of 0.05 per vertex; RPI.36), RPI.56's key over the exact fields only,
  and the stage output rules as diagnostics (RPI.68, RPI.69, RPI.75). C2's draft that every exact chart property is
  `not_established` now holds only when the validator's context carries no check result for it (RPI.48). C2's stage
  take-up adds a stage digest over the checked definition's debug text in place of the IR's canonical encoding
  (RPI.78), the checker's codes in their registered forms, the raster bound's pixel conventions (rounded outward,
  clipped, then widened to quads; a line's quad has no caps), one comma-separated `digests` entry in
  `stage_work_exceeded`'s context, and no frame report for a device fault or a later frame of a quarantined chain.
  C6's strip contract adds: a record states one relation over operands of one role with values of one role, each one
  canonical object ID; its level is the stored level it read, labels or poses; evidence references are opaque.
- Not built: the rest of CT-RPI-32, which needs the host to attach user stages (the core compiles and bounds them
  since C9 U5, WARP only, but the host attaches none yet; RPI.78); an exact checker of isometry and homomorphism for
  the stereographic chart, which is not a projective map (both stay
  `not_established`); charts and properties outside the drafted repertoire, the GPU agreement check (RPI.35) and a
  production precision rule (RPI.36). CT-RPI-08's rows J01 and J06 to J12 stay unsupported until a backend's own rows
  (C9); J10 stays an unsupported row with its payload built from M5's certified geometry and checked headless
  (`jumble::exposed_faces`); a decoder from the interpreter's values (the
  IR draft has list types since pull request 176); F05 and F26 stay unsupported, and the quarantine is not wired
  into the host,
  until the host attaches user stages (the core compiles and bounds them, WARP only, C9 U5); CT-RM-09's rows 3 to 6, 8
  and 10 wait for C9 for their device parts: the vertex evaluation and projection, U on a device and the skipped work
  (CT-RM-10, CT-RPI-31), and fault injection (CT-RPI-34); RM.38's weight table, device loss and `display_unsupported`
  belong to the renderer and its backend. The relationship strip's design, words and encoding tokens (EV3.8, EV5.11)
  and a strip record for a relation whose value is no canonical object are not built.
- Evidence kind: source and synthetic. Every ceiling in the tests is a fixture value; nothing is measured, and nothing
  advertises support or a profile (RPI.82, RM.40).

## Status of LP3, C4: module closure

- `c600-module` keeps a module's dependency closure, its lockfile and the approval bound to them (chapter 02 LG5,
  chapter 05 PS1.14 and PS1.15). `resolve` finds a manifest's closure among local files by content digest only and
  writes its lock. `verify` checks a manifest, its lock and the local bytes and gives the closure digest. `approve`
  checks the closure with `c600-check` for one entry, author, profile, grant and policy and binds an approval to
  the closure digest (RTE.45's `source_bound` variant for route RT1). `load` refuses a closure whose digest differs
  from the approval's, so substituted dependency bytes never run, and `Ready::run` runs the approved entry through
  `c600-run`'s boundary.
- Tests (`c600-module/tests/`): CT-RTE-08's source leg (`ct_rte_08.rs`, 15 tests): the approval binds every executed
  module; substituted, missing and mismatched dependencies are refused; record forms are checked before use; a moved
  file keeps its closure and its approval; dynamic code is refused; a program for another IR version or execution
  contract is refused with the checker's code in every author column; capability separation holds at the preview point,
  for a menu word's proposal as for a literal word's (LB1.8; LP2's pull request 210). CT-RTE-07 on 600-cell-Full
  (`ct_rte_07_full.rs`): 60 of LX-3's 75 frozen keys, each run ten times with one SHA-256 over its canonical result,
  engine side and store, and a check that every key is either realised or listed as left, exactly once. Among them are
  the priced budget keys B01 to B07, of which B01 to B04 and B07 run since pull request 354 under the price mapping that
  the crate documentation pins, and the four pick keys MR17, MR18, A29 and A30 since LP2's pull request 359. A pick key
  is two runs, since the draft language has no pick inside a run: the built-in solving list, whose block and
  `c600-pick` record the host keeps (FP.38, BND.54), then a proposal of the picked value. MR17 and MR18 pick from a
  list that is no fixed entry and are refused at preview admission with the corpus's code while BND.55 is open; A29
  and A30 pick a setup of the search for a named target, a fixed entry, and are admitted and previewed.
- Differences from the corpus, listed in the tests: A25 is a false refusal, which the corpus permits (separation
  refuses the preview of an allowed combination). P01 to P03 give the corpus's `LG3:capability.ambient_read` since
  the owner's decision of 16 November 2026 (PHS.52). A17's inverse of `D a` is `-1, +4`: the IR writes a letter's
  inverse as its negation, and `fixture.json` names `a` its own inverse, so the tests read `-1` as `a` and the value
  as the corpus's `a d`.
- Drafts, which freeze with the module formats (LG5.10, LQ10): the list in the crate documentation, among them the
  manifest and lock forms (`c600-module-draft` and `c600-lock-draft`, version 1), a null FP.11 identity for both
  records with the model identities in `payload.supports` (a gap for the format review), locations in the lock
  only, the closure digest, the in-memory approval record `c600-approval-draft`, and the separation refusal at the
  preview point, which stays in this crate until C5's host bridge. Dynamic code is `LG3:capability.not_granted` by the
  owner's decision of 16 November 2026 (PHS.52).
- CT-RTE-08's persisted words run in `c600-durable` (C7), on M6's store and through the command layer. CT-RTE-07's six
  jumbling keys run on M5's rows with literal words in `c600-jumble-tool` (C8, `tests/ct_rte_07_jumble.rs`); B10 and
  B11, a safety stop and a lost runtime, with a runtime process in `c600-tools` (C5, `tests/ct_rte_07_c5.rs`); B13 and
  B14, a missing budget and one over the host's ceiling, at the command layer on the 600-cell-Full tool's row in
  `c600-tools` (C5, `tests/ct_rte_07_budget.rs`); and MR13, MR14, MR19 and MR20, in which a state-derived read decides
  whether a later preview is emitted at all, are refused at the built-in column's admission on both executors in
  `c600-runtime` (`tests/ct_rte_07_admission.rs`): 74 of the 75 keys in all. B12 is recorded as run cold, and its
  other five cache states are not applicable while no host keeps a row cache (owner, 20 November 2026, item 1 of
  `../docs/wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md`). The six jumbling keys also run with menu words of
  the I_a alphabet, which `c600-jumble-tool` pins with its item matrices (C8, `tests/ct_rte_07_jumble.rs`). The Linux
  leg did not run.
- Evidence kind: source and synthetic. No timing is measured.

## Status of LP3, C3: the runtime in its own process

- `c600-runtime` runs a program in a child process of its own (route RT1, LQ9; RTE.21, ONE.69). The child binary
  reads framed messages on standard input, compiles and runs each job with `c600-syntax`, `c600-check` and
  `c600-run`, and forwards every host call (`tariff`, `perform`, `sink`) to the parent. The parent starts the child,
  answers its calls from the caller's host and returns the run's end or a typed fault. `run_in_process` is the same
  path in one process, the reference the tests hold the child to.
- On Windows the parent starts the child without a console window of its own (`CREATE_NO_WINDOW`, through the
  platform layer `c600-platform`): the shell has no console, and the parent starts a new child after every fault.
- Protocol draft `c600-rt1-wire-1`: length-prefixed frames of at most 16 MiB either way, one protocol byte, six
  message kinds, and a canonical value encoding that refuses every noncanonical form on both sides. The parent
  answers a repeated, unacknowledged call from its record and never calls the host twice for one call, and a new
  call must acknowledge the last answer, so at most one answer is open (KI.5). No message has a field for a token
  or any other authority (KN5.15). LP2's draft collections cross with one encoding each: lists, word keys (freely
  reduced), and sets and maps in canonical key order. A compressed word crosses as its parts and is rebuilt with the
  checked `from_parts` (LP2's pull request 185). A snapshot read with an argument crosses as
  `request.snapshot_with`, its argument as canonical bytes. A job may name a built-in tool's class rows
  (`JobOptions::built_in`, LP2's draft), a trailing field of the run: the child then admits the entry with
  `c600_check::admit_built_in`, and both executors refuse class rows outside the solving profile at the admission
  stage. A menu word (LP2's draft, LB1.8) crosses with its identity block (value tag 21, type tag 23), a menu-word
  preview as `request.propose_menu` (request tag 6), and a job may pin a menu alphabet (`JobOptions::menu`), a
  trailing field after the class rows: the identity block, the cap count, the inverse items and a matrices flag (0
  without the menu's item matrices, 1 with their count and each item's matrix; LG2.18, LP2's pull request 338), which
  the decoder checks as `MenuAlphabet::new` and `MenuAlphabet::with_matrices` do. `LB1:input.incompatible_alphabet`
  is one of the interpreter's own stop codes (`codec::STATIC_CODES`). An end test's verdict (LP2's draft, LB3.2)
  crosses as value tag 22 with its test's and
  its outcome's indices, and its type as type tag 24 with the test's index. A trace key (LP2's draft, LB1.10)
  crosses as value tag 23 with its header in LG2.23's order and its letters, and its type as type tag 25; a job may
  pin a trace relation (`JobOptions::relation`), the run's last field, after the pinned menu: the contract, the
  puzzle and model ID, the generator count and the independence bits, from which the decoder recomputes the digest,
  refusing what `Relation::from_parts` refuses. The codec cannot order a key's letters without the relation, so the
  run checks a decoded key against its pinned relation, and `LB1:input.key_contract_mismatch` is one of the
  interpreter's own stop codes. A menu word's trace key (LP2's draft, LB1.10, `jumble-locality/1`) crosses as value tag
  26 with its header (the jumbling identity block, the grip and item counts, the key, the contract and the digest) and
  its `[grip, item]` letters, and a pinned jumbling relation carries the menu's three identity fields after the model ID
  and the item count after the grip count. A menu word key (LP2's draft, LG2.18, `word-key/1`) crosses as value tag 27
  with its header (the identity block, the key kind, the grip count and the item count) and its tokens in their
  canonical form (each token's grip and the sixteen entries of its product row-major, each as its three integers), and
  the run checks its header against its pinned menu. A generator (LP2's draft, DET.61) stays in its run: it has no
  value tag, a value that holds one is refused whole, and its type crosses as type tag 27 (value and type tags are
  separate spaces; type tag 26 is F2 and F5's type, below). The lengths that a frame's compressed words derive are
  bounded together by one frame's bits, so one word of up to 16,382 doublings of a letter crosses.
  Above the bound a host's answer reaches the run in process but stops the child's run with
  `PS1:input.noncanonical_value`, a known difference (crate draft 15).
- LP2's draft F2 and F5: an element crosses as value tag 24, its field's byte (0 for F2, 1 for F5) and its residue's
  byte, and the field types as type tag 26 and the field's byte. An unknown field byte or a residue at or above the
  prime is refused on both sides. As an argument an element reaches the child, which refuses it as the run in process
  does, and a result stops both runs alike (`fields.rs`). The transfer form is no stored form: FP.6 has none for an
  element, and a host's answer to a read declared `F2` or `F5` crosses only in the transfer form.
- Faults: a child that is lost, does not start, names another protocol, sends a malformed or oversized frame or
  breaks the call protocol ends the run as `error` with `KN7:resource_limit.runtime_lost`; a passed deadline ends it
  as `cut_off` with `KN7:resource_limit.safety_stop`. The child is killed and reaped, and the next run starts a fresh
  one.
- LX-2's fault hook (the feature `lx2-faults`, off by default and in the product build): `child::main_with(hook)` runs
  the child as `child::main` does and calls `hook(answers)` on the interpreter's thread, with the answers the child has
  received, before each call frame and each result is written, next to where debug builds rehearse faults.
  `FAULT_HOOK` says whether a build has it. The faults belong to the binary that gives the hook (c600-lx2's injection
  device); this crate has none. The test child `c600-runtime-hooked`, built only with the feature, records the hook's
  calls (`fault_hook.rs`, run with `--features lx2-faults`; without the feature it checks that `FAULT_HOOK` is false).
- Tests (`c600-runtime/tests/`): the child and the in-process run give the same results, charges, stop points, host
  calls and outputs (`equivalence.rs`); CT-RTE-04 in part, 32 of DET.65's 40 traces as `.6cs` programs on both executors
  (`ct_rte_04.rs`; `nested-calls` and the two output traces since LP2's M2 meter, the exact-multiplication, alias and
  pointwise-failure traces since LP2's `exact_mul` and `assert`, three generator traces since LP2's `rng_init` and
  `rng_draw`, the five child traces, child records included, since LP2's child scopes, `final-loop-test` since LP2's
  loops, the two decoding traces since LP2's byte decoding, on bytes the host delivers as a snapshot, the three
  streaming traces since LP2's streamed words, the split and the coalesced trace as one job, since no host call or
  answer falls among a stream's rows, and `child-decode-error-releases-reservation` since LP2's bytes the code holds,
  `text_decode_hex`, so that the child's rows are its call and its decoding); the protocol's bounds, canonical
  encodings, KI.5's record, KN5.15's field list, each host request's round trip and its refusal when cut or unknown,
  the class rows, the pinned menu and the pinned trace relation as strict trailing fields, and a menu word's, a
  menu-word preview's and a trace key's encodings and refusals (`protocol.rs`); a snapshot read with an argument
  through the child, a built-in entry admitted, refused for a row its class does not list and refused outside the
  solving profile, and a pinned menu's words, refusals and preview, alike on both executors (`equivalence.rs`); LP2's
  draft collections and compressed words, their canonical forms and refusals on both sides, and their runs through the
  child (`collections.rs`); CT-RTE-07's MR13, MR14, MR19 and MR20, refused at the built-in column's admission alike on
  both executors, ten runs each under one SHA-256 (`ct_rte_07_admission.rs`); CT-RTE-10's headless fault half,
  rehearsals in debug builds that kill, crash, truncate, garble, oversize or hang the child while the session keeps its
  journal, state hash and pending preview (`faults.rs`).
- Drafts, which freeze with the IR or stay open past H-08's freeze: the list in the crate documentation, among them the
  framing and message kinds (KI.15, ONE.69), the transfer encoding (DET.59), acknowledgement and retry (KI.5), the
  fault codes (KN7.13), source bytes rather than checked IR in a job (KI.6, ADR-L12.9), and a deadline checked
  between messages only, so a blocking host call is not interrupted.
- Not built: OS isolation of the child (a job object with kill-on-close, a restricted or AppContainer token,
  filesystem and network confinement); a bound on the parent's queue of frames to a child that stops reading its
  input (draft 5); LX-2's and LX-4's decisive runs (C9, on the owner's machine). CT-RTE-04's other 8 traces need
  what the IR draft lacks: a call frame without a slot that holds bindings, text order, a flow check and an executor
  for `lp1-exec-region-draft-1` (LP2). At LP3's command-table freeze they leave LP3's run under PHS.52 and stay on
  record in DET.65's vectors (owner, 20 November 2026, items 3 and 4; DET.65), so CT-RTE-04 runs whole at 32.
- Evidence kind: source and synthetic. The fault rehearsals are not LX-2 evidence, and no timing is measured.

## Status of LP3, C5: the source route

- `c600-tools` holds the built-in 600-cell-Full tool as 600 Cell Script source,
  [`full_600_cell.6cs`](c600-tools/src/full_600_cell.6cs), in edition `6cs-draft-1` (syntax candidate A). Its one entry,
  the exported command `preview_word(w: Word)`, proposes the word the solver named to `operation.preview` and returns
  the acknowledgement's two fields as a record `{base, id}`. The base library's first entries, among them LB1's literal
  words, LB2's exact completion and LB2's recipes and reports, are language code in `c600-library` (section "Status of
  LP3, C5 library: LB1's and LB2's first entries").
- Admission: the checker admits the source with no diagnostic. The entry's only effect is `preview` and it requests no
  capability, so it prepares previews and reads no state (LG3.13, BND.39). `c600-check`'s `Profile::Solving` is the
  column of a solver's own tool, which prepares no preview until the joint setting. `Tool::new` admits the entry with
  `c600_check::admit_built_in`, LP2's draft of the built-in column of chapter 07 section 8 (pull request 170), and the
  class rows of `tool::class_rows`: the preview row of each row the manifest names, an LP3 draft, because BND.57's rows
  by class are not data. A variant that also reads state is refused with `LG3:capability.not_granted`. The class
  (`analysis`, `checking`, after LB6.5) and the row set are declared in the crate's Rust manifest, because the IR and
  the syntax carry no module manifest yet.
- The tool runs on `c600-run` through `Boundary` in the solving profile, in process: C5 was built before the
  runtime's own process (C3) reached main, and moving the tool onto it is C5's next piece. Against an engine double
  that records each proposal, the program receives only the fixed acknowledgement whether the engine built the
  preview, refused it, or preview admission refused the word, and its charges do not depend on the answer (BND.42,
  BND.43).
- `expand` is the runner's recipe expansion through the kernel, outside any program. Over O01 to O07 (3,201 steps)
  each step the oracle accepted or refused for protection expands to a literal word with the recorded
  `expansion_sha256` and `primitive_count`. Of O07's ten invalid steps, four are single words of integer moves, which
  pass to the tool unchanged; the other six have no literal word, so the route stops there with the kernel's refusal.
- The host bridge, `c600_tools::bridge::Bridge`, is the runtime's host on `c600-command`. Each proposal the boundary
  admits becomes one invocation of `operation.preview` with `Caller::Runtime` for the run, a fresh command ID
  `<run>/<n>`, the layer's current epoch, the literal word as the absolute argument
  `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}` and no token. For one run the host may give the bridge the
  solver's recipe, exactly as written, for any recipe the kernel expanded, star steps and word steps with other fields
  included: a proposal of exactly that recipe's literal word longer than 100,000 letters then goes as that recipe,
  unchanged, in the one invocation, and every other proposal in one word step; the bridge never cuts, splits or rewrites
  a word or a recipe (LB2.3, LB2.4). The program still receives only the fixed acknowledgement;
  the layer's receipt goes to the shell. Besides proposals, the bridge serves five reads, each to a snapshot read with
  an argument (`c600_tools::kernel_rows`): KN2.4's `positions` row since LP2's pull request 226; its `labels` row, the
  home slot of the sticker in each named slot, which LB2's exact completion reads; its `effect` row, a recipe's net map,
  which the kernel computes without reading the head and which LB2's recipes and reports read (pull request 294), and so
  do LB2's named macro analysis and seeds (pull requests 353 and 357); its `protection` row, a captured lock of exact
  slot and label pairs judged against a recipe at the head, Net at the end state and Strict at each prefix, which LB2's
  Net and Strict protection reads (pull request 300); and its `replay` row, a recipe replayed from the head prefix by
  prefix under the session's protected orbits, with Strict's first refused prefix and the orbits in conflict there,
  which LB2's named macro analysis reads (pull request 353). When the host gives it the certified setup table, it serves
  a sixth read, the `setups` row: the certified setups for a target the solver names, which only the setup-search
  exception may read (pull request 309). It serves the `orbits` row too, the pieces of named orbits, which LB2's
  orbit catalogue reads: model data, the orbit the kernel's `positions` gives each piece at the solved state, so it
  reads no state of the session (pull request 358). It also serves the three structure reads of LP3's
  representations tool (C6): `structure.compare`, two cells compared, `filter.preview`, the counts of a layer
  predicate, and `structure.layer`, its members, from LP2's structure rows (pull requests 262 and 375), which read no
  state. It serves the `journal` row too, KN4.3's page of the session's move tree, newest first, which LP3's C8
  reads, through LP2's `CommandLayer::journal`, which changes nothing (pull request 383;
  `c600-tools/tests/journal_row.rs`). Admission refuses an entry that both reads one and proposes (LQ21). It grants
  no other state read, declassified value, file or workspace access, view, exact sink or clock. The command layer's
  own rules refuse a runtime invocation of a commit or app row and any runtime
  invocation carrying a token (ADR-L02, KN5.15), and only a token the shell mints commits the pending preview the
  runtime proposed.
- The source route: the binary `c600-lp3-source` replays O01 to O07. Per step the runner expands the case recipe to
  its literal word, the tool's program runs on that word and proposes it through the bridge, and the shell presents
  the preview, commits it with the commit chord `Ctrl+Shift+Enter`, and undoes and redoes on the command route's
  schedule. `python tests/test_lp3_source_route.py` builds `c600-lp3-oracle` and `c600-lp3-source` and passed with:
  - `tools/oracle/compare.py`: all seven cases equal on the source route, 3,201 steps, no omitted field;
  - every source-route trace byte-identical to `c600-lp3-oracle --route command`'s;
  - the two routes' receipts agreeing on status, job, error code and revision after in all 13,549 pairs, and differing
    only in command ID, caller and request digest, with each source-route job number moved down by the solver's
    commands so far (3,195 of them, B2 below); all 3,191 preview-ready result digests agree under the command route's
    preview number, 289 of them once rebased onto the case recipe's draft, and the six O07 recipes with no literal
    word are the kernel's refusals on both routes;
  - a negative control: negating the first letter of O06 step 3's literal word before the tool runs makes compare.py
    fail at that step (`expansion_sha256`).

  CI runs this check through `tools/ci/run_headless.py`.
- Tests (`c600-tools/tests/`), each naming the conformance parts it covers; no conformance ID runs whole here except
  CT-LB2-04, in `exact_completion.rs`, CT-LB2-01, in `ct_lb2_01.rs`, CT-LB2-02, in `ct_lb2_02.rs`, and CT-BND-18, in
  `ct_bnd_18.rs`:
  - `tool.rs`: the source parses and lowers; the checker admits it and it only prepares previews (CT-LB6-03, part); a
    variant that also reads state is refused (LQ21; CT-LB6-03, part); a built-in previews only the word the solver
    named; a refused built-in, or a module with one checker diagnostic, never runs and never calls the engine, and a
    word from state relayed through a workspace record or a declassifier is refused (CT-LB6-03, part); the class and
    the rows are declared (CT-BND-15, part).
  - `ack.rs`: the fixed acknowledgement against an engine double (CT-BND-11 and CT-BND-07, parts).
  - `bridge.rs`: one runtime preview per proposal and the fixed acknowledgement on the command layer (CT-BND-11 and
    CT-BND-07, parts); no commit or app row for the runtime (ADR-L02, KN5.15); a runtime invocation carrying a token is
    refused (KN5.15); the runtime cannot set the protection policy (`protection.policy.set` sent by the bridge, or a
    tool variant's proposal to it, is refused with `LG3:capability.not_granted`, and the policy stays); only a token
    the shell mints commits the pending preview (CT-ONE-29, part, headless); the bridge grants nothing but a proposal
    and the reads of the `positions`, `labels`, `effect`, `protection` and `replay` rows.
  - `expand.rs`: every case step expands to its recorded literal word, and the expansion is the kernel's literal.
  - `positions_row.rs` and `labels_row.rs`: a library entry reads KN2.4's `positions` and `labels` rows through the
    bridge at the layer's head, in the asked order, and changes nothing (CT-LB2-04, part); each row refuses a declared
    type it does not answer and an argument that is not a list of integers before its charge, and a piece or slot
    outside the model with `KN2:input.named_object_invalid` once charged; its tariff (CM.18) covers its largest answer;
    the built-in column admits a reading entry only with the read row in its class.
  - `exact_completion.rs`: LB2's exact-completion entry from `c600-library`, CT-LB2-04 whole (section "Status of LP3,
    C5 library: LB1's and LB2's first entries").
  - `effect_row.rs`: a library entry reads KN2.4's `effect` row through the bridge, which answers a recipe's net map and
    what it touches without reading the head; the row refuses a declared type it does not answer and an argument in
    neither recipe form before its charge and keeps the kernel's refusal of a recipe once charged; its tariff (CM.18)
    covers its largest answer; the built-in column admits a reading entry only with the read row in its class
    (CT-LB2-01, part; the refusal with Jumble on is not run, because a jumbled layer needs `c600-jumble`).
  - `ct_lb2_01.rs`: LB2's recipes and reports from `c600-library`, CT-LB2-01 whole on 600-cell-Full against the
    differential oracle: every step of O01 to O06 reports the oracle's full-label effect, O07's invalid recipes answer
    no effect, with the kernel's code, the library's star words are the kernel's stars, a star's frames are exact
    ordered slot tuples, and an orbit selection reduces no support (pull request 294).
  - `protection_row.rs`: a library entry reads the draft `protection` row through the bridge at the layer's head: a
    captured lock of exact (slot, label) pairs at the level `labels`, judged against a recipe in Net or Strict mode (Net
    judges the end state at the head, Strict the base and each prefix of the literal expansion, up to an optional
    limit). The answer comes in its declared order; the row refuses a declared type it does not answer and another
    argument form before its charge, and, once charged, Jumble on, a recipe the kernel refuses and a slot or label
    outside the model; its tariff (CM.18) covers its largest answer; an entry that captures and judges in one run is
    refused by the built-in column (LB3.5) (CT-LB2-02, part).
  - `ct_lb2_02.rs`: LB2's Net and Strict protection from `c600-library`, CT-LB2-02 whole on 600-cell-Full through
    `Boundary`, the tool's bridge and a command layer. The captured locks (position, identity and block) hold the head,
    from solved and from a scrambled head. For each lock, two identity words (`[5, -5]` and a word of eight letters)
    pass Net (`clear`, one state checked) and fail Strict at prefix 1, with the conflicts in the lock's order, as the
    kernel's `piece_protection` and `strict_piece_protection` judge them; a check that stops short of the word's length
    answers `unknown` with `LB2:protection.incomplete_coverage` and never `clear` (pull request 300).
  - `replay_row.rs`: a library entry reads KN2.4's `replay` row through the bridge at the layer's head: a word, a list
    of integers or a list of step records replayed under the session's protected orbits as the kernel's Strict
    derivation judges it, at solved and at a scrambled head, with the first refused prefix (-1 with none protected) and
    the orbits in conflict there, and the whole word's net map digest as `effect` gives it; a read changes nothing. The
    answer comes in its declared order; the row refuses a declared type it does not answer and another argument form
    before its charge, and keeps the kernel's refusal of a recipe once charged; its tariff (CM.18) covers its largest
    answer; the built-in column admits a reading entry only with the read row in its class. The refusal with Jumble on
    is not run, because a jumbled layer needs `c600-jumble`.
  - `macros.rs`: LB2's named macro analysis and comparison from `c600-library` on the same host, each entry admitted
    only with the rows it reads in its class. A report answers the named macro's whole effect, the session's protected
    orbits, Net's conflicts and Strict's first refused prefix (an inverse pair passes Net and fails Strict at its first
    letter), the same at solved and at a scrambled head; a comparison answers the support both macros move and each
    moves alone and whether the net maps and the expansions are equal, from the kernel's effects (pull request 353).
  - `seeds.rs`: LB2's seeds from `c600-library` on the model's 35 seeds in `assets/seed_atlas.json`. `seed_report`, on
    the same host, answers each seed's whole effect: its target's pieces are the atlas's three-piece cycle, its support
    is the atlas signature and its collateral the atlas collateral, empty for 12 seeds. `stage_order`, with no host,
    puts every orbit before each orbit its seed moves, and returns the atlas's certified stage order unchanged (pull
    request 357).
  - `ct_lb1_02.rs`: CT-LB1-02's 600-cell-Full half, the library's word equality keys on the pinned model's relation
    `full-disjoint-support/1`: words with equal keys derive the same state from solved and from a scrambled state; at a
    blocked generator the literal `a, a⁻¹` and its key's spelling are both refused at their first letter under Strict
    protection, and the empty word is admitted; two words with one key are refused at different prefixes (pull request
    350).
  - `setups_row.rs`: the `setups` row through the bridge: its answer in the declared order (buffers, family,
    orbit, position, setups, table digest), its refusals and their costs, its tariff, which covers its largest answer,
    and the table's load through the kernel's asset check (CT-BND-18, part).
  - `ct_bnd_18.rs`: LB2's guarded setups and buffers from `c600-library`, CT-BND-18 whole on 600-cell-Full. The entry is
    admitted only under the setup-search exception and is refused under `[Read]` alone. Setup search runs only for a
    named target: a stand-in with a literal or computed target, a setup picked from the answer, an emptiness test or a
    proposal is refused with `LG3:capability.not_granted`. The setups keep both buffers fixed in every prefix, for every
    orbit and at least 70 targets, against an independent read of the tree asset, and a search changes nothing. A buffer
    position has no setup and still names the family. The solver's edit of a setup is a word it builds itself, and
    nothing previews until its own command (pull request 309).
  - `structure_rows.rs`: the draft `structure.compare`, `filter.preview` and `structure.layer` rows through the bridge
    (LP3's C6): an entry reads two cells compared, the counts of a layer predicate and its members (the positions it
    counts and their home slots, pull request 375) as LP2's structure rows give them, in the declared order and with
    `Nat` or `Int`, the same in a solved and a scrambled session; each row refuses a declared
    type it does not answer and another argument form before its charge, and a cell outside 0 to 599 or a layer past
    the cell's last with `KN2:input.named_object_invalid` once charged; the tariffs (CM.18) cover the largest answers.
    The refusal with Jumble on is not run, because a jumbled layer needs `c600-jumble`. Covers no conformance ID.
  - `orbits_row.rs`: the `orbits` row through the bridge (pull request 358). Each named orbit's pieces agree
    with the kernel's orbit table: the `positions` row, read at a scrambled head, gives each of them that orbit, and a
    moving orbit's count is the kernel's progress row's and the seed atlas's. The answer comes in the argument's order,
    repeats included, and in the declared order; the row refuses a declared type it does not answer and another
    argument form before its charge, and an orbit outside -1 to 34 with `KN2:input.named_object_invalid` once charged;
    its tariff (CM.18) covers its largest answer; the built-in column admits a reading entry only with the read row in
    its class. A read changes nothing. The refusal with Jumble on is not run. Covers no conformance ID.
  - `orbits.rs`: LB2's orbit catalogue, progress per orbit and residuals from `c600-library` on the same host (pull
    request 363). The built-in column admits the catalogue with the `orbits` row, and a host entry that reads `orbits`
    and `positions` at what the solver names with both rows; it refuses one that reads `positions` at the catalogue's
    answer (LB3.5). The catalogue is the kernel's orbit table at solved and at a scrambled head; the progress is the
    kernel's progress row on all 36 orbits at a scrambled head; the residuals agree with an oracle over the certified
    setup table's buffers, and at solved every orbit is complete and solved while a partial or reversed list is neither.
    The largest orbit, 7,200 pieces all away from Home, runs within the budget ceiling, and the test prints what
    it used. Covers no conformance ID.
- Open: the class rows are an LP3 draft until BND.57's rows by class exist as data. The built-in column's two exceptions
  (setup search for a named target, the repair list) are built in `c600-check` (LP2's pull request 229); the bridge
  serves the `setups` row only when the host gives it the setup table. The command layer's receipts carry no program
  digest; the proposal's provenance block names the digest of the source bytes that ran.
- Drafts, which freeze with the IR or stay open past H-08's freeze, are in the crate documentation under "Drafts".
- Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg was not run. The timings the
  source-route check prints are informational, not performance evidence.

## Status of LP3, C5: the input router

- `c600-input` routes synthetic key events in EV4.4's fixed order: a key-capture field, a text field, a focused list
  or view taking only its navigation keys, then the global map. A key that reaches a binding has its relative names
  resolved at the press and goes to the command layer through `c600-command`'s public shell API with absolute
  arguments only (EV4.3). A held key repeats only EV4.6's allowlist (`camera.rotate`, `camera.zoom`, `list.move`,
  `view.preview.scrub`), read from the row's H-08 `repeat` flag; focus loss releases the held keys and a pending
  prefix group; busy input gets the layer's refusal as its receipt and is never queued. `KeyMap::apply` runs EV4.7's
  duplicate, prefix and layout checks per routing class and per declared layout (US QWERTY and French AZERTY) before a
  map applies. The crate changes no file in `c600-command`.
- Tests (`c600-input/tests/`), on synthetic events only:
  - `routing.rs`: CT-EV4-02, among them arguments resolved at the press surviving later camera, focus, hover and
    filter changes, and the key route giving the same receipts as `Shell::invoke` with the same rows and arguments.
  - `repeat.rs`: CT-EV4-04's append, preview, commit and twist legs and its busy leg, with repeat events flagged and
    unflagged, counting the invocations that reach the layer. A held twist key commits once through `twist.apply`
    against the pinned menu; a twist key-down that meets a held commit lane gets the layer's refusal, and nothing
    commits on its behalf when the lane frees.
  - `twist.rs`: the twist keys' exact requests (A6, KN3.7) at four caps and in the draft destination; with Jumble on,
    the key's rotation is `jumble.twist.menu`'s letter; CT-BND-19's live-twist part, with the router joined to the view
    model and the tool's view of `c600-tools`: until the twist key's key-down commits the twist, no field of the view
    model holds a value of any twist a key could release at the fixture's four grip slots (the view model's only input
    is the layer's bundle, and the layer has not moved), and after it Global, Local and the tool's view show the
    committed state.
  - `keymap.rs`: CT-EV4-06's synthetic half, ahead of LP4 (chapter 12 section 15.5), among them DT-M1's US and French
    layout counterexample.
- Drafts as of H-08's freeze (PHS.53), which wait for chapter 05's key-map forms: the list in the crate
  documentation, among them the routing class names, the chord form (W3C `code` names with Ctrl, Shift, Alt and
  Meta), the two layout tables, the default global map, the repeat flag's source, placeholder camera values, and the
  twist keys: `KeyJ`, `KeyK` and `KeyL`, with Shift for the inverse, type items 1, 2 and 3 of the pinned menu in the
  cap's fixed frame, which settles nothing in the chapters (G6 owns the keys). Escape closes a pending prefix group or
  the innermost transient layer and never cancels a preview (DT-M1 C4).
- Open:
  - The class rule (no text-class or capture-class binding; list-class bindings reach only the list rows) reports
    `EV4:input.binding_conflict`, since chapter 13 has no row of its own for it; a dedicated row is a change to
    chapter 13 (PHS.52).
  - Closed by LP4-8: two draft key maps existed (the shell's five chords and the router's default map). One checked
    map remains, in `c600_command::keys`, and Escape cancels no preview.
  - The host gives the router the pinned menu's two digests, since the command layer gives no read of its pin; a read
    in `c600-command` would remove the copy. No key binds `jumble.twist.menu`, and K2's other four axes, a slot's
    transported frame (D11) and C6's seat keys have no key.
- Evidence kind: source and synthetic. No Windows, IME, device or timing claim; CT-EV4-05 stays a windows test (LP4).

## Status of LP3, C7: durable receipts on the session store

- KN5 has one implementation, owned by LP2 under PHS.18 (integrator, 16 November 2026), in `c600-command`, which keeps
  its `Store` trait, with `c600-session` (KN4) implementing the trait; M6's semantics hold where the frozen text leaves
  a choice. Its stages 1 and 2 landed in pull request 174 (c1664c3d): `CommandLayer::open` replays a store's history,
  and `c600_session::JournalStore` is the layer's store over KN4's journal (C1's section above); stages 3 to 5 landed in
  pull request 194 (5809dfea). `c600-durable` is a test-only crate that drives the command layer over `JournalStore`,
  with C5's provenance reader installed, M6's session store and C4's module closures through their public APIs, and
  changes none of them.
- Tests (`c600-durable/tests/`):
  - `ct_rte_08_persisted_word.rs`: CT-RTE-08's persisted-word case, its durable leg (BND.22, BND.84, BND.85, LG5.1,
    LG5.6). On the layer over the journal, an approved own tool stores a word, which a program cannot deliver, and the
    solver writes it through the layer's `workspace.set` with the boundary's provenance block. After a power loss on a
    memory disk, and in a new session on a file-system disk in a fresh folder under Cargo's test directory, the record
    and its stored receipt come back equal (KN5.23); the block names the module the approval binds and no other; the
    layer's preview of the record as a `{"record": name}` draft is refused with the reader's BND.84 refusal and changes
    nothing (KN5.26); and a solving run that proposes the word is refused at admission, with nothing reaching the
    engine. The same word stored by the entry approved as a built-in tool fails closed on its missing part (BND.85)
    until the host keeps the run's blocks again; then its record draft is previewed and presented, a write of another
    record keeps the preview fresh, and a later write of that record makes it stale (`KN5:stale.revision_changed`), so
    the press prepared before commits nothing. That write keeps the word's part with the record (CT-BND-27; LP2's pull
    request 272), so after another power loss a host that keeps no block admits the record's preview.
  - `kn5_on_layer.rs`: CT-EV3-07 on the layer over the journal (EV3.2, EV4.15, KN5.20). A commit, the solver's
    `workspace.set` and a commit take places 1 to 3 of one `session.undo` order; a hint taken before a later action of
    either kind is superseded; undo, then redo, walk the three actions in order across both kinds, each after a power
    loss. With the host's view model behind EV3.2's view rows (C5's `ModelViews`), the solver opens and closes a view of
    a language overlay row of kind `view`; the view's run is refused `workspace.set`, `session.undo`, `view.open` and
    `view.close`; and the row publishes from the view's run and from the shell, none of it writing the journal or making
    the undo hint stale.
  - `kn5_on_session.rs`, on M6's own `Session` API: CT-EV3-07's durable action order (a commit, a `workspace.set` and a
    commit take orders 1 to 3, an undo hint taken before the set is refused, the order and all three receipts survive a
    crash, and undo runs 3, 2, 1) and its view part on M6's stand-ins (a query job, an opened snapshot and a preview
    leave the order, the undo hints, the revision and the journal bytes unchanged, and a program's or a job's workspace
    write is refused); CT-LB2-03's work-sheet part (setting a named sheet again makes the old preview stale, and a new
    proposal reads the new content, also after a reopen).
  - `ct_lb2_03_work_sheet_reuse.rs`: CT-LB2-03's work-sheet sentence through the layer's `worksheet.save`,
    `worksheet.load` and `worksheet.compare` rows on the journal, with C5's reader installed (LB7.2, LB7.3, LB2, KN0.8,
    KN5.23). A sheet saved from a draft record comes back equal at a reuse in the same session, after a power loss on a
    memory disk and in a new session on a file-system disk; each reuse makes a presented preview of the sheet stale, so
    the press prepared before it commits nothing, and a fresh engine preview commits the sheet's complete operation; a
    reuse after that commit places the piece roles again and shows the moved piece without retargeting the sheet, the
    position role on the moved piece's home reads the piece that fills it now (LP2's pull request 361), and the
    reuse's undo restores the sheet. A comparison of a sheet with the current work completes as a query whose result
    digests it, writes nothing, the journal's bytes included, and keeps a presented preview fresh; its fresh analysis
    of the sheet's complete operation gains nothing, loses the positions the operation unfinishes, as LB2 counts them,
    and ends where the operation committed inline ends; a sheet saved from a record of letters reports that record as
    its current macro version. After a power loss on a memory disk the comparisons come back equal, and a later write
    of the record of letters makes the macro version changed while the sheet's own word is still the one analysed.
    Each refusal of the three rows that LP2's draft names comes back with the layer's code, queues no job and changes
    nothing, the journal's bytes included: no reader, in a session reopened without one; the runtime; a malformed
    argument; an unknown or removed record, a record with no operation to save, no saved sheet to reuse or compare, or
    no work to compare with, a malformed sheet among them; another version; and a block that C5's reader refuses. The
    refusals of the save and the reuse with Jumble on run in `c600-command`'s `tests/jumble_on.rs`, not here: entering
    Jumble on needs `c600-jumble`, which is no dependency of this crate. A comparison while the exclusive lane is held
    runs in `c600-command`'s `tests/worksheet.rs`.
  - `saved_sets_on_journal.rs`: the durable leg of LP2's saved-set drafts (`set.save`; U1-057, U4-037, PS1.28,
    KN5.23; LP2's pull request 337) on the layer over the journal with C5's reader, the row served as the host serves
    it at every open and the save's job finished with the entry's record, where the host runs the representations
    tool's `save_set` (`c600-repr`'s `tests/served_rows.rs` runs the entry). A save's receipt and the saved set's
    record `set/<name>` come back equal after a power loss on a memory disk and in a new session on a file-system
    disk, and the exact-set digest follows from the record read back. The save is undoable (H-08's undo class
    `workspace` since the command table's freeze, LB7.2), after a reopen too: an undo of a replacing save gives back
    the set it replaced, and the next undo removes the record write before it. An identical
    retry answers the stored receipt; a save under a taken name without `replace: true`, a `replace` that is no
    boolean and a `workspace.set` under `set/` are refused; a save whose name another save took meanwhile fails at its
    completion; none of these writes the journal. A save with `replace: true` replaces the set, also after a power
    loss.
- CT-LB2-03 runs whole, across crates. Its first sentence, recipe chunking and the refusal of an oversized recipe, runs
  in `c600-kernel` and `c600-tools` ("C5 next" below, which records chunked transport on the source route for any recipe
  the kernel expanded); its second sentence, named work-sheet reuse
  invalidates the old preview, with LB7.2's fresh exact analysis of a reused sheet (LP2's `worksheet.compare`, pull
  request 277), runs on the layer's memory store (`c600-command`'s `tests/worksheet.rs`) and on M6's journal (above).
- Drafts, in the crate documentation: the shell's `workspace.set` as the host's delivery of a stored word (the word's
  letters as a JSON list, the boundary's block as text; C5's runner writes no record); C5's reader installed under the
  fixture's pins, the host keeping a run's blocks again after a reopen, where the layer gives back only the part blocks
  it kept with the writes; a built-in tool's stored word for the fresh and stale preview, since preview admission
  refuses a solver tool's word (BND.84); a language overlay row of kind `view` standing in for a view's evaluation on
  the layer; M6's stand-ins for views on M6; and a test reader that reads every block, standing in for a host's reader
  that accepted a block C5's reader refuses, so that the journal replays such a record and a sheet saved from it.
- Built by LP2 since: KN5.26's admission matrix L1 and L2 runs through the layer in `c600-command`'s
  `tests/kn5_admission.rs` (pull request 270); a durable home for a word's part blocks (pull request 272), which C5's
  reader serves with `Provenance::parts` and `admit_kept` (the persisted-word test above, and `c600-tools`'
  `tests/workspace.rs`); and EV3.2's view rows (pull request 271, landed through 272), which `c600-view`'s view model
  serves through C5's `ModelViews` (the CT-EV3-07 view test above). LB7.2's fresh exact analysis of a reused work sheet
  (`worksheet.compare`, with LB2's gains and losses) and macro versions (pull request 277) run on the journal above. The
  session's profile (BND.14 to BND.17, pull request 287): the layer asks the reader to admit a record's word only in a
  solving session, as KN5.26 asks. A reuse reads each position role's occupant again, and a comparison names the roles
  whose occupant changed (pull request 361; the journal test above). M6's session lock covers `JournalStore` too
  (LP2's pull request 195). `kn5_on_session.rs` still drives M6's own `Session::invoke` and shell stub, which the
  layer supersedes and LP2 removes once their tests run against it.
- Evidence kind: source and fixture. The file-system leg is a short test in a fresh folder, not long-session
  evidence. The checks ran on Windows only; the Linux leg was not run.

## Status of LP3, C5 next: the process host and the tool's views

- The process host, `c600_tools::process::ProcessHost`, runs the 600-cell-Full tool in `c600-runtime`'s child (C3; route
  RT1, RTE.21). The child gets the source bytes, the manifest's module and entry, the job's word, the run limits of the
  job's budget and the tool's class rows, and no token, secret, grant or command layer; the boundary and the bridge stay
  in the caller's process as the runtime's host, so the program still receives only the fixed acknowledgement. One child
  serves every run, and after a fault the next run starts a fresh one (KN7.13). The job runs in the solving profile with
  those class rows (`JobOptions::built_in`, LP2's draft), so the child admits the entry in the built-in column itself,
  as `Tool::new` does in the caller; LP3's hosts start a run only from a command in a solving session (BND.17).
- Each run starts inside a command the solver invoked (finding B2 of the KN5 boundary attack, ONE.34, the owner's
  reading of 18 November 2026; draft), on both hosts. The tool's row `tool.full_600_cell.preview_word`
  (`c600_tools::process::TOOL_ROW`) is a query overlay row whose one argument, `w`, is the word, served by the tool's
  entry. The shell invokes it with the literal word, the layer admits a program job, and the host takes the job
  (`invoke_tool`) and runs the entry as the job's run, `job-<id>`, which the bridge stamps on the run's invocations.
  Once the preview the run proposed is built, the host reports the job with the program's acknowledgement
  (`ToolCommand::acknowledge`), so the solver's command completes as a query; a run refused at its start stage is
  reported as the job's failure with the refusal's code (`start_error`). The layer's check, `c600-command`'s
  `SOLVER_COMMAND_CHECK`, is on (the KN5 readings above), and every LP3 route and test runs this way.
- Each run is under its job's budget (KN7.7; LP2's pull request 348; draft), on both hosts. The solver's command
  declares the budget, by default the shell's, `Budget::CEILING`, and the layer admits it under its ceiling before the
  job exists; a command with no budget, or with one over the ceiling, is refused there, so the tool does not run. The
  host runs the entry under the run limits of the job's budget (`ToolCommand::limits`, through
  `c600_tools::tool::limits`, which maps the layer's four dimensions onto the meter's by name); the ceiling gives the
  tool's own limits, `tool::LIMITS`, and the process host sends them in the child's job
  (`process::job_under`). The host runs the job's word (`ToolCommand::word`) at the job's revision
  (`ToolCommand::base`). The bridge declares the run's budget on the run's invocations of a row an entry serves
  (`Bridge::with_budget`; without it, `Budget::CEILING`).
- `c600-lp3-source --host process` replays O01 to O07, and `python tests/test_lp3_source_route.py` runs both hosts:
  compare.py equality on all seven cases, traces byte-identical across the hosts and to the command route's, and step
  records, receipts and callers equal across the hosts. On the source route each step that gives the tool a word adds
  two of the shell's receipts, the solver's command running as a new job and its report under the same command with
  the acknowledgement's digest, and every job number is the command route's moved up by the solver's commands so far.
  Four O07 steps (0, 1, 2 and 8) have a letter outside 1 to 1200 in magnitude, which the runtime's wire cannot encode:
  on both hosts the run is refused at its start stage (`PS1:input.noncanonical_value`; in process `run_in_process`
  refuses such a word as the parent does) with no invocation, and the solver's command fails with that code; the step
  record keeps the kernel's refusal, which the check compares with the command route's receipt's error.
- The tool's views: `c600_tools::views` is host code, since the tool publishes no view (LQ21, LG3.13, charter E6). It
  builds the display contract's publication of a proposal and its preview on `c600-display`, the content the headless
  view model installs on `c600-view`, and the presentation report: each composed frame goes to the command layer
  through the shell as a `PresentedFrame` with the display's stamp and the chain that drew the preview, and the
  layer presents the pending preview only for a frame the built-in chain drew (KN5's stage 5).
- The host's view rows, `c600_tools::views::ModelViews` (EV3.2, RPI.81 to RPI.83; LP2's pull request 271), is the
  host's `c600_command::Views` on `c600-view`'s view model. The layer admits `view.open` and `view.close` and calls the
  host once per command ID; the model applies its view namespace, RPI.82's budget and the view lane. Each layer command
  ID becomes the model's caller command, numbered in the order the host first sees it, so a superseded run's cancel
  reason names the command that closed its view. The host's declare function gives each source's dependencies,
  workload, output allowance and class, and refuses a source the host does not serve before the model is called (the
  tests' hosts with `KN5:input.argument_form`, as `ModelViews` refuses an overlay digest that is not 64 hex digits). A
  closed view's resources stay charged until the host takes its retirement token at the frame boundary and completes
  it (RM.19).
- The host's provenance reader, `c600_tools::workspace::Reader` (KN5's stage 3; PS1.16, KN5.26, BND.84, BND.85), is the
  host's `c600_command::Provenance`. Its check reads a record's block with `c600_run::provenance::read`, so
  `workspace.set` refuses a block that is not a canonical FP.17 block; its admission of a `{"record": name}` draft is
  `c600_run::boundary::admit_preview` over the block, the blocks the host keeps and the part blocks the session keeps.
  At a record's write it lists the parts of the record's block that the host keeps, recursively (`Provenance::parts`),
  which the layer keeps with the action and gives back at admission (`Provenance::admit_kept`; CT-BND-27, LP2's pull
  request 272), so a reopened session admits a record written while the host kept its parts. It takes every session as
  a solving session, the only profile LP3's host runs, fails closed on a part that neither the host nor the session
  keeps, and drops no kept block. `c600-lp3-source` serves H-08 with the layer's draft rows and installs the
  reader, which keeps each run's blocks; the runner writes no record, so its traces and receipts are as before.
- Chunked transport on the source route (CT-LB2-03's chunking part; LB2.3, LB2.4, LB1.11) carries the solver's recipe
  exactly as written for any recipe the kernel expanded, star steps and word steps with other fields included. The host
  gives the bridge the input recipe with its literal word (`c600_tools::bridge::Steps`, made only from the kernel's
  expansion), and a proposal of exactly that literal word over 100,000 letters goes as
  `{"draft": {"recipe": <the solver's recipe>}}` in its one invocation, so the source route admits it exactly when the
  command route admits the same draft. Every other proposal goes in one word step, and the bridge never cuts, splits or
  rewrites a word or a recipe.
- Tests (`c600-tools/tests/`), each naming its part:
  - `views.rs`: CT-EV3-01, CT-EV3-02 and CT-EV3-04, whole for the tool's view; CT-EV3-03, whole, its imported half on a
    fixture import record; CT-BND-19 for the tool's views (node text, derivation, derived subject, preview literal and
    installed bytes hold only the word the solver named; its live-twist part runs in `c600-input`'s `tests/twist.rs`
    on this view model and view); CT-ONE-29, whole, at the host's adapter through the layer's `present` (the layer
    refuses a user-stage frame, a frame with no preview and a frame with another preview, so no token is minted and
    the commit chord finds no fresh preview; a built-in-chain frame's token binds its stamp and commits; a later
    user-stage frame ends the presentation and a delayed older report changes nothing).
  - `view_host.rs`: the command layer's view rows on the view model through `ModelViews`. CT-EV3-08's admission part
    through the layer: the solver opens a view from the tool's built-in source and one from an overlay row of kind
    `view`, each `accepted` with no job, and the model holds both with their sources and schedules each first run; a
    close supersedes the closed view's first run, whose cancel reason names the closing command; an open past RPI.82's
    budget is refused with the field and fits once the closed view's retirement completes; nothing is written.
    CT-RPI-36's model half through the layer: the model's refusals reach the receipt with the view as detail, a source
    the host does not serve is refused before the model, and only a run inside a command the solver invoked reaches
    the model. A retried delivery reaches the model once, and the host takes only 64 hex digits as a digest, a sign
    excluded, even from a caller that bypasses the layer.
  - `workspace.rs`: a record write whose block the reader refuses is refused with the reader's error and writes nothing,
    and a block that reads is written whatever its admission would say (PS1.16, BND.21); a record draft is admitted or
    refused by the reader's admission: a hand-entered word passes, a solver tool's word is refused, and a built-in
    tool's word fails closed on its missing part until the host keeps its run's blocks, each refusal installing no
    preview (KN5.26 and CT-KN5-13, part; CT-BND-29, part); and a record's write keeps the parts of its block that the
    host keeps, recursively, so a reader that keeps nothing admits the word with the parts the layer keeps, while on
    its own it fails closed (CT-BND-27, part).
  - `process.rs`: the same summaries, values, shell records, invocations, receipts and layer on both hosts, each run
    inside the solver's command and reported after its preview is built (B2); the job carries only the admitted source,
    in the solving profile with the tool's class rows, and the child refuses it at admission without the class rows,
    with a class that leaves out `operation.preview` or in the tool-authoring profile; both hosts refuse a word with no
    transfer encoding at the start stage and fail the solver's command; both run the tool under the budget its command
    declared, so a budget of one step cuts the run off with the meter's steps code before it proposes anything, with the
    same summary on the two hosts, and the job reported as failed with that code changes nothing; a runtime that does
    not start is reported and changes nothing; the binary names the host in every record (CT-BND-11 and CT-BND-07,
    parts).
  - `ct_rte_07_budget.rs`: CT-RTE-07's B13 and B14 on the tool's row, in the corpus's tool-authoring profile: a command
    that declares no budget, or that declares the corpus's two steps under a host ceiling of one, is refused before
    admission with the corpus's code; the receipt names no job, the layer queues none and the tool does not run; the
    labels, the revision, the journal, the protected slots and the pending preview stay as they were, and the session
    stays out of read-only recovery; a command that then declares a budget at the ceiling is admitted as job 1. The
    preferences and the recovery checkpoint, which the corpus lists too, belong to M6's session store; the command layer
    holds neither.
  - `net_oversize.rs`, on both hosts: CT-LB2-02's Net half (an identity word whose prefixes move protected orbits
    previews clear in Net mode and commits as one transaction, and the prefix alone is refused) and its Strict half
    (after the shell's `protection.policy.set` to Strict, the same word previews with the kernel's Strict row, verdict
    moves, `mode: "strict"` and `first_refused_prefix: 1` in its summary and no end state, as on the command route, and
    its commit is refused with nothing changed; with no orbit protected it previews clear and commits as one
    transaction); CT-LB2-03's oversize refusal (100,001 letters refused whole on both routes with the same code,
    100,000 admitted) and its chunking part: on the command route, and on the source route a recipe of two full chunks
    previews in the solver's two word steps with the command route's digests and end state and commits as one
    transaction, and so does a recipe over one chunk with a star step or a word step with another field, sent as the
    solver's recipe, unchanged; a word over one chunk that is not the input recipe's literal word, or comes without the
    solver's recipe, is refused whole with the recipe-shape code, and a star recipe's literal word of at most one chunk
    goes in one word step.
  - `lb3_checker.rs`: CT-LB3-07's checker half, as far as capability separation decides it (an outcome table that reads
    state and prepares previews is refused; a named-word check that answers only the acknowledgement is admitted).
  - `kn5_retry_lead.rs`: a lead on KN5.21 to KN5.23, closed by LP2's KN5 implementation (ruling 4, owner decision of
    16 November 2026). A refused envelope that reuses a genuine `operation.execute` envelope's command ID, arguments
    and binding (a token minted with another secret, or the same envelope on a runtime's channel) no longer becomes
    that command ID's outcome: its refusal is kept under its full digest, the genuine envelope commits under the ID,
    and neither retry commits anything more.
- Not built: an import row; grip, cell or piece objects for a word's letters (the kernel has no letter-to-grip map); a
  view declaring both of the manifest's classes (`c600-view`'s `Declared` holds one class); a view's evaluation for the
  view model's scheduled runs (the host running the view's program and installing its content); named work-sheet reuse,
  which C7 runs on the layer. CT-LB3-01's checker half and CT-LB3-07's recommendation rule are `c600-check`'s (LP2, pull
  request 213).
- Drafts, which freeze with the IR or stay open past H-08's freeze, are in the crate documentation under "Drafts".
- Evidence kind: source and synthetic. No Windows, DirectX, input or timing claim; the source-route check's timings are
  informational. The checks ran on Windows only; the Linux leg was not run.

## Status of LP3, C6: the representations tool, headless half

- `c600-repr` is LB6.5's row "Representation, work-set and filter tools", class `view_aid`, over LB4's layouts and
  atlases, map property checks and linked selection. It draws nothing and owns no device, renderer, clock or session:
  picks resolve through `c600-display`'s host and the installed source map, exact chart checks are `c600-display`'s, and
  each entry's admission is `c600_check::admit_built_in` with the class rows of `tool::class_rows` (an LP3 draft: the
  snapshot reads, the published view and the workspace records the shipped entries use).
  - `layout`: sheets and atlases as display plans with their source maps; a layout filter rebuilds the layout, the
    instance list and the map's entries and leaves the mechanically analysed set alone (LB4.4); a linked selection is
    computed once for its inputs, and frames only read it (EV3.10).
  - `chart`: a chart's survey over camera poses, with exact verdicts at exact poses from `c600-display`'s checks and
    display data between them (LB4.2), and a user chart's admission as far as `c600-display` reaches it (LB4.3).
  - `tool`: the source [`representations.6cs`](c600-repr/src/representations.6cs) (edition `6cs-draft-1`, syntax
    candidate A) with six entries, each admitted under the built-in column; the Rust manifest with the class
    `view_aid`; the review of the class against the exact entry definitions and the row set, which also finds an
    exported function the manifest does not list and source bytes other than the ones the manifest names; and the form
    of a review record that names every entry digest (BND.37).
  - `rows`: what each of the six rows needs from the command layer, what the layer meets of it and what remains. LP2
    built the four common needs in the layer: a dispatch from a stored row to a built-in tool's entry
    (`CommandLayer::serve_row`, pull request 200), argument forms (pull request 202), the runtime's view and workspace
    sinks as the job's result (`JobOutput`, pull request 203) and a workspace revision that moves. A host serves all six
    rows with their entries; the layer serves `filter.apply` since pull request 256. C6 changed no file in
    `c600-command`.
  - `applied`: the applied filter's display. It reads the record `filter.apply`, the filter preview the layer keeps
    for it and the records that preview read, and resolves the layer predicate to its members with LP2's
    `CommandLayer::layer_members` (pull request 375). A representation's layout then keeps the members' slots,
    stickers, pieces and certificate points and nothing else, and its analysed set stays (LB4.4). A write of a record
    the preview read, a newer filter preview that replaced the applied one, and a reopen each leave the layout
    unfiltered, with the reason.
- Tests (`c600-repr/tests/`), each naming its part:
  - `ct_lb4_03.rs`: CT-LB4-03's 600-cell-Full half. Display instances of two charts map through the installed source map
    to occurrence, sticker, piece, cell and slot; a chart's inverse value is never consulted for a pick; a layout filter
    preserves the analysed set. The jumbling half, at pose level, is C8's (`c600-jumble-tool/tests/ct_lb4_03.rs`).
  - `ct_lb4_charts.rs`: CT-LB4-01's CPU reference through the survey and the admission (a declared domain decided by an
    exact region test and section membership on the posed vertices, boundary cases included; a chart whose vertices pass
    and whose domain fails inside is refused; an established domain establishes no injectivity; a flat cell's embedding
    is established by its own check while the stereographic chart's isometry and homomorphism stay `not_established`; a
    full-dimensional region whose vertices all pass the vertex test fails injectivity inside, and the chart is refused);
    CT-LB4-02, whole (synthetic: replacing the decisions between exact poses changes only display data); CT-LB4-04's
    admission half and CPU half.
  - `tool.rs`: the source parses and lowers, and every entry is admitted; CT-BND-15's representations part; CT-LB6-03's
    representations part (a stash-and-relay, a state-chosen target, a writing view and an exported function the manifest
    does not list are each found). The record is checked with a fixture reviewer: BM5's review of the tool happens when
    it ships.
  - `ct_ev3.rs`: CT-EV3-06 whole, in three parts: the counter half (unchanged frame draws and a workspace-only
    revision compute no relation; counted, never timed); the strip record, C8's current occupant of every slot on the
    I_a menu carried as `c600_display::strip` records (total when solved; after a twist off the lattice, "none: off
    the lattice" and never the old occupant; `unknown(reason)` where two pieces share a chamber; KN0.4's three forms;
    `strip::attach` shows a record only on a relation node of its own revision whose claims carry its statement key);
    and the strip's schedule (the record is computed once at open and once for a burst of two state revisions, at the
    newer, and never per frame or for a workspace-only revision). CT-EV3-07's view half at the command layer (the
    checker refuses a view that writes the workspace or proposes; after a commit, program calls to the six rows and to
    `session.undo` are refused, a view overlay row completes as `view_published`, and the revision, the log and the
    undo hint stay as they were).
  - `rows.rs`: the requirements name the stored rows as H-08 has them, each row's effects are its entry's, the layer
    refuses all six rows until a host serves them, an overlay cannot replace one, and `serve_row` accepts exactly the
    rows the list marks served, all six since pull request 256.
  - `served_rows.rs`: a test host serves the six rows with the tool's entries and C5's provenance reader, and runs each
    job's entry in process through `c600-run`'s boundary. Invoked from the shell, `filter.preview` and
    `structure.compare` complete as `completed(query)` and `view.effect.scope` as `completed(view_published)`, at the
    job's revision and with nothing written; `filter.apply`, `set.save` and `structure.layer` each complete as one
    durable workspace action at a workspace revision one higher. `filter.apply`'s and `structure.layer`'s write the
    entry's record through the reader; `set.save`'s writes the saved set's own record `set/<name>` with the exact-set
    digest as the result reference (LP2's pull request 337, `c600_command::sets`).
    `filter.apply`'s entry takes the layer's `FilterPreviewRef` as a record of its epoch and job. Each of the three
    actions is an undo entry (H-08's undo class `workspace` since the command table's freeze, LB7.2): undo takes them
    back newest first, `set.save`'s, `filter.apply`'s, then `structure.layer`'s, each removing its record and moving
    no state, and then the commit before them. The source
    route (the same entry run in process on the same arguments and state, without the layer; not C5's runner) gives the
    same value, views and records (for `set.save`, the saved set its record names, with the exact-set digest). On both
    routes the entries pass their arguments to the two snapshot reads
    (`compare_cells` the two cells, `filter_counts` the layer predicate the `FilterRef` names, as the record its job
    wrote), and the engine side answers them on the session's layer with the kernel rows of `c600-tools`' bridge, so the
    views are the kernel's step count and layer counts. The layer refuses malformed and stale arguments before any job.
    A test checks `filter.apply`'s refusal of a stale filter preview. An invocation of the two queries and the view with
    `Caller::Runtime`, as a running program's, carries the provenance block the host builds from the run's context
    (`Boundary::block`), through LP2's `invoke_with_inputs` (pull request 264): C5's reader checks it, the job's run
    takes it, and the row completes as from the shell. Without blocks the host fails such a job with
    `LG3:capability.not_granted` and runs nothing, and a block the reader refuses is refused before any job. No program
    operation reaches a served row (LG3.1; a program reaches a row only by a proposal, KN5.9), so the test host stands
    in for the runtime's adapter. Since LP2's pull request 237 the layer refuses a workspace row such as `set.save` on a
    running program's channel with `LG3:capability.not_granted` before any job (the owner's narrow reading of ONE.52, 18
    November 2026: a run never writes the workspace). A test applies the filter of a layer predicate through the
    command route, and the display shows a sheet's member slot, sticker, piece and certificate point and nothing else,
    beside the analysed set it was given; after a commit it still shows them, since the members read no state. A new
    write of the predicate, a newer filter preview, a reopen and an undo of the apply each leave the sheet unfiltered
    with the reason. The members' refusal with Jumble on is not run.
- Built since: the host rows for the `structure.compare` and `filter.preview` snapshot reads, on LP2's structure rows
  (pull request 262), in `c600-tools`' bridge (`c600-tools/tests/structure_rows.rs`). LP2 built the
  `CanonicalObjectRefs` form (pull request 269); `tests/served_rows.rs`'s `[3, -5]` case is an admission refusal since.
  LP2 built a record and an exact-set digest for each saved set (pull request 337): `set.save` writes `set/<name>`
  (its kind, its members ascending and distinct, the model identity, the name and the capture revision) and gives the
  exact-set digest as the result reference (`c600_command::sets`). At the freeze H-08 takes `replace: Bool?` and
  chapter 13 registers `KN5:input.saved_set_exists` (CM.17); names under `set/` are reserved. LP2 built the members
  of a layer (pull request 375), and the
  applied filter's display reads them (`applied`, above).
- Not built, LP3's next steps: a reference type in the IR, so `filter_counts` takes the predicate as a record of its
  cell and layer (the two reads and their tariffs are H-08's since the freeze, CM.18); a display that applies the
  published scope (its API is in the crate documentation under "Not built"); a display of an applied filter whose
  filter preview the layer no longer keeps, after a reopen or once a newer one replaced it, which needs the applied
  predicate kept with the record or by the layer; every device step (CT-LB4-04's rest, C9; the stage checker is
  LP2's since pull request 241); the relationship strip's design and its other typed relations (C8 computes one, a
  slot's current occupant, at pose level, and `c600_display::strip` carries it); filters of trees, complexes and
  graphs; a BM5 review record.
- Drafts, which freeze with the IR or stay open past H-08's freeze, are in the crate documentation under "Drafts".
- Evidence kind: source and synthetic. No Windows, DirectX, input or timing claim. The checks ran on Windows only; the
  Linux leg was not run.

## Status of LP3, C8: the jumbling tool, headless half

- `c600-jumble-tool` is LB6.5's row "Named twist/word checker and kept checks", class `checking`: PHS.22's built-in tool
  for jumbling. Its source [`jumble_checker.6cs`](c600-jumble-tool/src/jumble_checker.6cs) (edition `6cs-draft-1`,
  syntax candidate A) has two entries, `check_named_twist` and `check_named_word`, each admitted under the checker's
  built-in column through `c600_tools::tool`. Each proposes the literal word the solver named and reads no state, so
  the tool never chooses or outputs a word by itself. `word` reads a `Word` as literal jumbling letters, cap then item,
  of the state's menu; a word that is no sequence of letters is refused before any letter is applied.
- The crate also holds LB1.8's literal words of one pinned menu (`word`: concatenation, inverse, conjugate `s w s⁻¹`,
  commutator `a b a⁻¹ b⁻¹` and powers, written out with no cancellation; a mix of alphabets is refused with
  `LB1:input.incompatible_alphabet`), the two levels a result reads (`level`: poses inside Jumble, labels in
  600-cell-Full), protection verdicts that answer only at their own level (`protection`; asked at the other,
  `KN1:input.level_not_held`) and one typed relation of the relationship strip at pose level, a slot's current occupant
  on and off the lattice, each lattice piece placed by M8's slot action of K⁺ (`relation`; EV3.8, EV3.9).
- The crate's `library` holds the jumbling library entries as 600 Cell Script on the IR draft's menu words (LP2's pull
  request 210) and its F2 and F5 (LP2's pull request 222): sixteen modules, each in the formatter's canonical layout
  with its catalogue row. `menu_words`, `menu_traces` and `menu_word_keys` read no state; each other entry reads only
  its rows with `read_snapshot`, so it reads state and prepares no preview. None names a proposal target, and none
  chooses or outputs a word, except at the one point the owner allowed (17 October 2026): at a cap the solver names,
  `jumble_repair` lists the pairs whose three-twist repair lowers N, in a neutral order, and the solver picks one.
  - LB1.8's literal words of one pinned menu ([`menu_words.6cs`](c600-jumble-tool/src/menu_words.6cs): `power`, beside
    the IR draft's own `concat`, `inverse`, `conjugate` and `commutator`; pure).
  - LB1.10's trace keys of menu words ([`menu_traces.6cs`](c600-jumble-tool/src/menu_traces.6cs): `same_trace` and
    `trace_commute` under the relation `jumble-locality/1` that the crate's `locality` pins beside the menu alphabet,
    from the table M5 regenerates from the loaded model; pure).
  - LB1.8's word equality keys of menu words, the jumbling half of "Word equality keys"
    ([`menu_word_keys.6cs`](c600-jumble-tool/src/menu_word_keys.6cs): `key` and `same_word`, LG2.18's domain-preserving
    key `word-key/1` under the item matrices that `word::Alphabet::pinned` pins with the menu alphabet; pure).
  - LB3.1's "Heights, branches, domains and Φ" ([`jumble_heights.6cs`](c600-jumble-tool/src/jumble_heights.6cs): Φ, the
    largest left height and the pieces at heights (0,0) and on the lattice's domain, by exact sums over KN2.4's
    `heights` and `height_counts`).
  - LB3.1's "Release and whole-word checks" ([`jumble_word_checks.6cs`](c600-jumble-tool/src/jumble_word_checks.6cs):
    the chronological observations and the step flag, recomputed from KN2.4's `replay` and `word_check`).
  - LB3.1's "Protection" at pose level ([`jumble_protection.6cs`](c600-jumble-tool/src/jumble_protection.6cs): a whole
    word's Net and Strict verdicts from KN2.4's `protection`, M8's `levels::derive`).
  - LB3.1's "ν and drafted correction-set check" ([`jumble_nu.6cs`](c600-jumble-tool/src/jumble_nu.6cs): ν's 45 values
    in F2 and F5 from KN2.4's `nu`, and the change the solver's drafted set makes, with `correction_set_check`).
  - LB3.1's "Rim and seam facts; N" ([`jumble_rim.6cs`](c600-jumble-tool/src/jumble_rim.6cs): every cap's row in cap-ID
    order from KN2.4's `rim_summary` and one whole `seam_facts` scan, with exact counts; total N is the scan's).
  - LB3.1's "Covers at a named cap" ([`jumble_covers.6cs`](c600-jumble-tool/src/jumble_covers.6cs): both directions for
    every cap that meets the solver's cap, with no top-cap verdict).
  - LB3.1's "Correction catalogue" ([`jumble_corrections.6cs`](c600-jumble-tool/src/jumble_corrections.6cs): the
    correction the solver names by its id; it chooses no combination).
  - LB3.1's "Admissibility and blocked-twist check"
    ([`jumble_admissibility.6cs`](c600-jumble-tool/src/jumble_admissibility.6cs): the solver's named twist, a menu word
    of one letter, from KN2.4's `classify`, and the solver's named grip from `survey`).
  - LB3.1's "Straddlers and u at a named cap" ([`jumble_straddlers.6cs`](c600-jumble-tool/src/jumble_straddlers.6cs):
    each straddler with its exact certificate pair, and u).
  - LB3.1's "Units and clean-step contract" ([`jumble_units.6cs`](c600-jumble-tool/src/jumble_units.6cs): every off unit
    or the solver's named unit, and the contract's parts for the solver's word and unit).
  - LB3.1's "Repair list at a named cap" ([`jumble_repair.6cs`](c600-jumble-tool/src/jumble_repair.6cs): LB3.6's list as
    read, under the built-in column's repair-list exception).
  - LB3.1's "Retained checkpoint, label checkpoint, sticker test"
    ([`jumble_end_tests.6cs`](c600-jumble-tool/src/jumble_end_tests.6cs): each verdict as its own type, LG2.9; the
    retained checkpoint takes the solver's witness word and builds none).
  - LB3.1's "Mode crossing prerequisites"
    ([`jumble_crossing.6cs`](c600-jumble-tool/src/jumble_crossing.6cs): `jumble_off_facts`, A5's prerequisites of
    Jumble off, the label checkpoint's outcome, the pieces off the lattice and ν; `jumble_on_facts`, Jumble on's
    journal derivation, the entrance the head's line reaches in KN4.3's `journal` through retained commits only; it
    decides no crossing).
- The crate's `rows` is the jumbling rows' host bridge (pull request 299): it serves KN2.4's jumbling rows to a run as
  snapshot reads, at the command layer's head with Jumble on (`Rows::at_head`), on a witnessed base (`Rows::on`) or at a
  state (`Rows::at_state`). It uses the crate's draft answer types and H-08's tariff (CM.18; one pass of the model's
  pieces for the state and one for each letter of a named word or entry of a drafted set, two for a named cap's seam
  facts), and it grants those rows only. Since pull request 306 it also serves J08's `seam_facts` for a named cap, which
  CT-RTE-07's jumbling keys read.
- `c600-display` gains the jumbling producers (`jumble`): the posed-sticker row J02 from a state's canonical pose table,
  the certified transform J03 with the plane the engine names for the twist (`twist::Twist::plane`; a jumbling twist has
  a swept preview, a half-turn at θ = π in the engine's direction), the certified overlays J04 (the full named-cap set,
  the cap-map witness and the named cut), J08's seam table from the engine's `seam_facts`, and the slot projection J13,
  which places a sticker at any lattice pose by M8's slot action of K⁺ with A3's frame check
  (`levels::Tables::piece_slots`), with a fixture support report for J02 to J05, and, since pull request 301, the
  producers of J01, J06, J07, J09, J11 and J12; J10's payload (`jumble::exposed_faces`) is the instanced layout
  `magic600-jumbling-exposed-faces/1` built from M5's certified geometry: base faces in binary32 with their fan
  triangles, the 7,200 K⁺ matrices, each piece's transport and every face's piece, base face, pole, side and cell,
  checked by `jumble::check_exposed_faces`, with RM.12's check of its input bytes against a supplied ceiling
  (`jumble::admit_exposed_faces`; no numeric ceiling is fixed yet, RM.13); a refusal of M5's producer passes through. Under a jumbling identity
  the validator now checks a posed row instead of refusing it; 600-cell-Full is unchanged.
- Tests, each naming its part:
  - `c600-jumble-tool/tests/tool.rs`: CT-LB6-03's jumbling part (the declared class against the exact entry definitions;
    LX-3's correction-solve, state-based target and storage-laundering cases refused under capability separation and
    never run) and CT-BND-15's jumbling part (the class, the row set `operation.preview` and a manifest that names the
    source digest).
  - `c600-jumble-tool/tests/lb3.rs`: the kernel halves of CT-LB1-02 (at a blocked cap literal a,a⁻¹ is refused at its
    first letter and the empty word is admitted; prefix protection reads literal letters; and its key half, LG2.18's
    word key under I_a's item matrices: the key of a,a⁻¹ at a blocked cap is the identity letter's token there, not
    the empty key, both words are refused there and the empty word admitted, in Rust and through the library entry
    `menu_word_keys`, and on random words from three states each word and its run-merged form have one key, one
    admissibility and one end digest; and the entry itself: canonical, pure, and refused by the checker with
    `LG2:type.word_key_not_executable` where letters are required, so CT-LB1-02 runs whole with C5's half),
    CT-LG2-13's jumbling half (the blocked cap has a nonempty straddler set; whole by union with the empty-word halves
    in `conformance_lg2.rs`), CT-LG2-12's jumbling fixtures under LG2.14's four hypotheses (the key merges only
    adjacent same-grip runs and keeps identity tokens, I_a is closed under products, and surveys at three states
    classify exactly; from solved some a,a⁻¹ and the identity letter have one key and end at solved, yet a,a⁻¹ peaks
    at a greater left height, and with a piece that a moves protected a,a⁻¹ is refused at its first letter while the
    identity letter is admitted; whole by union with the language and kernel halves), CT-LB3-01 (ten fixtures on
    M8's sticker test, retained checkpoint and label checkpoint, each pair of the three answering differently on some
    fixture; an imported table unestablished at the label checkpoint, and yes at the retained checkpoint only by the
    word its caller supplies; the ν case: a lattice state reached from solved under I_a whose ν, from LP2's `nu` row,
    is nonzero fails the label checkpoint, and one whose ν is 0 passes it), CT-LB3-03 (heights (0,0) off the lattice in
    free mode; domain IDs by exact coset tests; Φ = 0 with height-2 pieces fails M8's sticker test), CT-LB3-05 with
    CT-BND-17 (whole with the checker half, the entry `jumble_repair` and LP2's job, below; M8's repair list at a named
    cap: its pairs in pinned order, then by cap; rows of five integers with no
    mark and no ranking field; every pair's delta read again from its own word, and y and z recomputed for one pair; a
    stopped list has no rows, and an empty list speaks only of its cap), CT-LB3-06 (prefix peak, excursion, tameness and
    the step flag recomputed apart from the kernel's batch; a false flag refuses nothing) and CT-LB3-07 (a named-word
    check answers only its named word, with no word, outcome table or recommendation field; its correction part: a
    correction the solver names by its id is checked as its literal word only, LP2's `nu` and `correction_set_check`
    rows answer with ν's 45 values and the label checkpoint with ν and its source, and no answer holds a list of lists
    or a recommendation, ranking or correction field).
  - `c600-jumble-tool/tests/ct_lb3_07.rs`: CT-LB3-07 on the inspector entries `jumble_rim`, `jumble_covers`,
    `jumble_corrections`, `jumble_admissibility`, `jumble_straddlers`, `jumble_units`, `jumble_repair`,
    `jumble_end_tests` and `jumble_crossing` in the language, with the named-word checks and the drafted check, through
    the bridge on witnessed snapshots of the I_a walk (pull requests 308, 314 and 321). Each read is recorded with its
    argument. The rim entry answers every cap in cap-ID order and nothing of a twist. The covers and catalogue entries
    read the solver's named cap or id only, and the catalogue refuses another menu. No answer names a recommendation, a
    next cap, a ranking or a selection. The admissibility entries answer the solver's one letter or named grip only, the
    blocked twist with the engine's exact certificate; the straddlers entry answers the named cap's straddlers with
    exact certificates, and u; the units entries answer every off unit or the named unit, and the clean-step entry the
    named word and unit only, its flag LB3.8's. The repair entry (CT-LB3-05's and CT-BND-17's entry part) is admitted
    only under the built-in column's repair-list exception as written: refused for a literal cap, a helper's read, a
    field of the read and a condition on it. The survey row answers the named grip only: asked for every grip or for a
    list of grips, the run stops with `KN5:input.argument_form`, and the bridge refuses the same arguments (pull request
    332). The crossing entry, the Jumble-off half of "Mode crossing prerequisites", reads the label checkpoint, every
    off unit and ν once each and answers A5's prerequisites of Jumble off, the checkpoint's outcome alone, with no
    correction and no word. Its Jumble-on half reads only the `journal` row; `tests/jumble_on_derivation.rs` runs it.
  - `c600-jumble-tool/tests/ct_lb1_03.rs`: CT-LB1-03's jumbling kernel half (the constructors build words from letters
    alone, with no state in existence; the alphabet refusal for another menu, another pinned order and free mode; a
    word and its inverse retrace an admitted path to its start, prefix by prefix; a constructed conjugate gets no
    admissibility guarantee and is refused at the prefix that straddles, at the first or a later one).
  - `c600-jumble-tool/tests/library.rs`: the library's modules checked by LP2's checker and run by LP2's interpreter,
    each result compared with the kernel halves; the host bridge `rows` serves the KN2.4 rows (pull request 299).
    CT-LB1-03's jumbling half in the language (`menu_words` pure and in canonical layout; `power` and the IR draft's
    constructors build the kernel half's words in a run with no host; a menu word of the S4 menu, of I_a in another
    pinned order or in a run with no alphabet pinned refused before the run with `LB1:input.incompatible_alphabet`, as
    the checker refuses a `Word` where `power` takes a `MenuWord`, and the rows refuse a word at a state of another
    alphabet; a built word and its inverse retrace an admitted path prefix by prefix; a built conjugate refused at its
    first prefix), CT-LB3-03's library entry (per-pose heights, branches, domain IDs and counts, Φ and the largest left
    height equal to the kernel's on six menu-reached states; Φ = 0 with height-2 pieces fails the sticker test; in free
    mode every piece reads heights (0,0) while the moved pieces form one pose class off the lattice, with the exact
    coset's domain ID) and CT-LB3-06's library entry (peak, excursion, tameness and the step flag recomputed from the
    rows, equal to the kernel's; a false flag refuses nothing, also with the unit's unmoved pieces protected; a whole
    word's Net and Strict protection at pose level equal to `levels::derive`'s reports).
  - `c600-jumble-tool/tests/ct_lb3_02.rs`: CT-LB3-02 whole (centre piece 0 under retained generator 1, each verdict an
    engine's own: refused at pose level inside Jumble by M8's protection of a whole word, `levels::derive`, and clear at
    label level in 600-cell-Full by the kernel's protection of named pieces, centres included,
    `Kernel::piece_protection` and `strict_piece_protection`, each under Net and Strict; the crossing at solved through
    M8's label and retained checkpoints and `levels::project`; the label pass replaces nothing, so the word is refused
    again inside Jumble, and the turn Jumble on replays shows only at pose level; each verdict refused at the other
    level with `KN1:input.level_not_held`; each level finds both clear and moving cases). M5's
    `c600-jumble/tests/crossing.rs` runs the kernel facts as CT-KN2-03.
  - `c600-jumble-tool/tests/ct_lb3_04.rs`: CT-LB3-04 whole at C8's level, on LP2's `nu` and `correction_set_check` rows
    (`c600_jumble::nu`, `c600_jumble::corrections`): ν undefined (`off_lattice`) at every off-lattice prefix of a
    correction's word and after one letter outside K⁺, never stale, zero or partial; the C5 convention (r0 the least
    element of order 5, r0^i of class i), correction 27's value read in r0, unchanged through its 45 values, JSON and
    the label checkpoint's evidence, and the manual's rule "make correction 27 v times" checking to 0 in r0 only; a
    planted nonzero fixed I_a coordinate (b5, on the I_b menu with I_a's identity written over its own) refused by the
    row and the check with `KN2:consistency.fixed_nu_nonzero`, while an open claim and the true I_b menu bind nothing;
    drafted sets checked as drafted, each answer the catalogue's prediction as ν's 45 values, and a set with an
    unshipped id refused whole. Three tests run LB3's entry `jumble_nu` (`library::NU`, `src/jumble_nu.6cs`) in the
    language on the IR draft's F2 and F5 (LP2's pull request 222), through the host bridge `rows` on a witnessed base
    (pull request 299): off-lattice ν undefined, the planted refusal, and drafted sets checked to the prediction with
    the change Σ m·row.
  - `c600-jumble-tool/tests/rows.rs`: the library entries of CT-LB1-03, CT-LB3-03, CT-LB3-04 and CT-LB3-06 through
    `rows` at the head of jumbled sessions (I_a, S4, I_b), each run equal to the same run on a witnessed snapshot, byte
    for byte; with Jumble off, every read stops with `KN5:input.menu_word_needs_jumble_on`; the bridge's grants and
    tariff (CM.18); the `seam_facts` row at a named cap, whose N_c equals the engine's rim summary (pull requests 299
    and 306); the rows `classify`, `survey`, `straddlers` and `units`, each for the solver's named letter, grip, cap or
    pieces only (pull request 314); the `repair_list` row, which serves the named cap's completed list in neutral order
    with no outcome table, rank or mark; and the three end-test rows, each with M8's verdict, level, reason and
    evidence, read by a program as that test's own verdict type, the outcome alone (`LG2:type.mismatch` for another
    test's row; pull request 321).
  - `c600-jumble-tool/tests/ct_rte_07_jumble.rs`: CT-RTE-07's six jumbling keys (MR01, MR02, MR05, MR06, A31 and A32) on
    M5's rows with literal words, each re-expressed in `.6cs` and run ten times through `c600-module`'s boundary as the
    Full leg runs its keys, with one SHA-256 over the canonical results, the rows read, the proposals and the store; a
    manual-proposal control and the accounting against the Full leg (pull request 306). Each key and the control run
    again with menu words of the I_a alphabet, pinned with its item matrices, and give the same answers.
  - `c600-jumble-tool/tests/ct_ev3_06.rs`: CT-EV3-06's off-lattice half (all 259,800 slots on states of the I_a menu and
    on raw pose tables, against M8's labels: the occupant at the identity and at other lattice poses; "none: off the
    lattice" and `unknown` as two values in one result, `unknown` only where lattice pieces collide; never the old
    occupant; the coverage and missing part of a partial result kept; a slot outside the model refused).
  - `c600-jumble-tool/tests/ct_lb4_03.rs`: CT-LB4-03's jumbling half (eight display instances of six objects resolved at
    five levels on the pose-level subject; a 600-cell-Full label projection holds no pose level, so it answers
    `KN1:input.level_not_held` for it, also under that subject's own digest; a planted chart inverse is never called;
    four layout filters, one by pose, keep the analysed set, the posed row and the subject; at lattice poses other than
    the identity, a sticker's slot through `c600-display`'s pose projection on M8's slot action of K⁺, checked against
    the current-occupant relation, also for two pieces that share one pose index; and at a later revision of a jumbled
    session, with Jumble on in the command layer: a layout evaluated at a head the layer reaches by presses of a W-J
    journal's records, evaluated again at the revision the next press makes, where a piece comes onto the lattice, and
    at the one its undo makes, while the installed earlier evaluation goes stale and a pick under its binding is
    refused with `EV3:stale.display_pick`).
  - `c600-jumble-tool/tests/jumble_on_layer.rs`: oracle equality with Jumble on in the command layer (PHS.23's jumbling
    half; LP2's pull requests 212, 217, 218 and 220). For each W-J fixture (I_a, I_b and S4) one session enters Jumble
    on at solved and presses J1's journal records through `jumble.twist.menu` beside M5's engine: at every step the
    press commits, the revision advances by one, its digest is M5's digest of the prefix and the pieces whose pose
    changed are M5's inside set; at J1's stages the head has J1's digest, counts and posed-row hashes (J02); at the end
    the posed row and the current-occupant relation (CT-EV3-06) answer as on M5's state. The default run takes the
    first 130 records of each journal, across two of the layer's flat replays, and reaches J1's start stage; the whole
    journals, with the mid and end stages, run on request (`--ignored`). Not compared there: `jumble.off`, the refusals,
    undo and redo, protected pieces and free mode, which `jumble_layer_verdicts.rs` runs beside the tool's own entries
    (below).
  - `c600-jumble-tool/tests/jumble_layer_verdicts.rs` (pull request 323): Jumble on through the command layer beside the
    tool's own entries at the layer's head (`Rows::at_head`), in five tests on the W-J fixtures. `jumble.off` commits
    exactly when `jumble_end_tests`'s label checkpoint holds; it is refused with its fact's code when the checkpoint
    fails, and as not shown reachable when it is unestablished. After an exit every tool read stops with
    `KN5:input.menu_word_needs_jumble_on`, and undo, redo and undo again bring the jumbled head back with equal reads.
    Each admissible press moves as many pieces as `jumble_admissibility`'s twist check names, and a blocked one is
    refused with the code `word_protection` stops with for the letter; every refusal changes nothing. With an orbit
    protected, under Net and Strict, a press commits or is refused as `word_protection` answers. Redo brings back each
    head with its digest and the tool's reads, and a free binding at the next record's grip is refused with
    `MENU_BINDING_MISMATCH`. At each exit `jumble_crossing`'s prerequisites agree with the layer: the checkpoint's
    outcome, the head's pieces off the lattice and the `nu` row's ν; a committed exit has them all, and a walk of I_a
    (the retained word [1, 3, −5] and correction 2) reaches the refusal for a nonzero ν. The tests read the layer's
    decisions and add no rule of their own.
  - `c600-jumble-tool/tests/jumble_on_derivation.rs`: CT-LB3-07's Jumble-on half, Full labels with journal derivation.
    `jumble_crossing.jumble_on_facts` runs through `c600-tools`' bridge on a layer with Jumble off and on, and reads
    LP2's `journal` row (pull request 383) from the head. At each head its answer agrees with `jumble.on` pressed there:
    the solved witness after commits from solved, the exit root after `jumble.off` with and without commits (the exit's
    event and mark, and the letters replayed), Jumble on after the entry and a pose move
    (`KN5:input.retained_tool_needs_jumble_off`), and no entrance in a session that started away from solved
    (`KN5:input.jumble_on_labels_only`); undo and redo move the head and the answer with it. Each read sends no
    invocation and changes nothing; read one event a page, the entry pages on through the cursor to the same answer, and
    stopped after two pages it reaches no entrance.
  - `c600-jumble-tool/tests/jumble_layer_repair.rs` (pull request 339): the repair list through the command layer,
    LP2's job that names its cap (pull request 326), beside the tool's entry `jumble_repair` at the layer's head on
    the I_a walk, in four tests. At a named misaligned cap, at closed caps, at a cap whose completed list is empty,
    after a press that drops the kept list and after its undo, the job's list and the entry's read agree on status,
    input digest, parameters, message and rows; the job alone words an empty list (BND.2), each run of the entry
    reads the `repair_list` row once with the named cap, and neither side picks a cap. The tests read the layer's
    and the tool's answers and add no rule of their own.
  - `c600-jumble-tool/tests/ct_lb1_06.rs`: CT-LB1-06's jumbling half (LB1.10) on the model in `assets/`: the relation
    `jumble-locality/1` that `locality` pins agrees with M5's table on every pair of grips; bca and cab key to bca and
    abc to itself; identity letters and an item beside its inverse stay in the key; words with one key are admitted
    alike and end at one digest on M5's states, while an identity letter at a straddled grip is refused where the
    empty word is admitted; `menu_traces` checks as pure code and answers as the keys do; and a key under another
    menu, contract revision, key version or relation version refuses comparison with
    `LB1:input.key_contract_mismatch`; and "Word keys remain separate" on the jumbling side: in one run under I_a's
    item matrices and the relation, `word_key` (LG2.18, LP2's pull request 338) and `trace_key` key the same menu
    words each by its own rule, and a source that compares the two keys is a type mismatch. With LP2's 600-cell-Full
    half, CT-LB1-06 runs whole.
  - `c600-display/tests/jumbling.rs`: CT-RPI-08's jumbling rows (produced and validated, J03's transform with a swept
    preview whose R(θ) is the twist; unsupported and missing data explicit, a twist other than the identity with no
    plane and J10's refusals from M5's producer among them; J10's payload against the exact geometry, with corrupted
    payloads and forged answers refused), CT-RPI-12's off-lattice part on a real jumbling state
    (with C2's stale half, CT-RPI-12 runs whole), RPI.23 and J13 at lattice poses other than the identity (every
    sticker in the slot of M8's labels, a turned sticker resolved through the host to the slot k·s of the slot action,
    and an engine error of the level tables no projection) and CT-RM-07's jumbling half on M5's model (all 15 rows;
    row 2's half-turn sweeps at π; row 14, which A5 keeps from every witnessed I_a state, runs on CT-LB3-04's planted
    menu, I_b's with I_a's identity), so CT-RM-07 runs whole with C2's half.
  - `c600-display/tests/jumbling_payloads.rs`: CT-RPI-08's producers of J01 (render/1's pinned shrink anchors), J06 (the
    readback check), J07 (domain and lattice masks; the rim map and counts), J09 (cover tables and repair lists), J11
    (word and release checks and the release log) and J12 (ν and the correction catalogue, the predicted ν equal to the
    engine's `correction_set_check`), pull request 301.
- Taken up from LP2 since pull request 323: through the command layer, `jumble.off`, the refusals with Jumble on, redo,
  protected pieces and free mode, each beside the tool's own entries at the layer's head
  (`tests/jumble_layer_verdicts.rs`, above); since pull request 339, the repair list as LP2's job that names its cap
  (KN5, pull request 326), with BND.2's wording of an empty list and no key that skips caps with an empty list,
  beside the entry `jumble_repair` (`tests/jumble_layer_repair.rs`, above). C8 uses the IR draft's F2 and F5 (pull
  request 222) for LB3's ν entry (`src/jumble_nu.6cs`, `library::NU`).
- Not built: J01 and J06 to J12 as supported rows (a backend's own rows, C9);
  the jumbling rows' program-job route in the command table (KN5; H-08's `reads` name and price the rows since the
  freeze, CM.18);
  LB3.1's other rows as language code ("Menus and pinned letters" and "D and release accounting"; thirteen LB3.1 rows
  are built, under `library` above,
  and "Correction-combination solve" is a research tier candidate), and LB3.8's named cap with ΔN, Δn_h, Δu, the moved sets and the prefix digests; BM5's review
  records; the relationship strip's design (its data contract is C6's `c600_display::strip`, which carries the
  current-occupant relation in `c600-repr`'s CT-EV3-06 tests). The posed arrays at the J1 fixture's mid and end
  stages are compared only in the whole-journal runs, on request. The survey's all-items loop is not served (LB3.5):
  `survey` answers the solver's named grip only. F03 on 600-cell-Full has no swept preview: the retained kernel names no
  plane for its twists, and which engine names one is not settled. The language entry `jumble_repair` reads the
  named cap's list under the built-in column's repair-list exception (LP2's pull request 229) and runs as no job of
  the layer (the program-job route, above); LP2's job serves the same list (pull request 326, taken up above).
- Drafts, which freeze with the IR or stay open past H-08's freeze, are in each crate's documentation under "Drafts".
- Evidence kind: source and synthetic. No Windows, DirectX, input or timing claim. The checks ran on Windows only; the
  Linux leg was not run.

## Status of LP3, C5 library: LB1's and LB2's first entries

- `c600-library` holds the base library's first entries as 600 Cell Script (LB1.1): chapter 03's row "Literal words"
  (LB1.8), the F2, F5 and Q(√5) sign parts of LB1.8's row "Exact linear algebra", the quaternion factor and tree part of
  its row "Quaternion and icosian algebra", its row "Word equality keys" for 600-cell-Full words, the exact-completion
  half of LB2's row "Residuals and exact completion", LB2's rows "Star and recipe construction", "Collateral and
  support", "Net and Strict protection", "Guarded setups and buffers" and "Named macro analysis and comparison", and
  parts of its rows "Seeds and full seed analysis" and "Orbit catalogue and collateral stage order". Its source
  [`words.6cs`](c600-library/src/words.6cs) (edition `6cs-draft-1`, syntax candidate A, in the canonical layout) is the
  module `words` with one exported pure function, `power(w: Word, n: Int) -> Word`: `w` written out `n` times,
  `inverse(w)` written out `-n` times for a negative `n`, and `word[]` for `n = 0`. A power is literal: nothing cancels.
  Concatenation, inverse, conjugate and commutator are the IR draft's own operations (LG2.15), so the library adds
  powers and wraps none of them. A client imports the library by the SHA-256 of its source and is lowered with it.
- The catalogue row with LB1.2's five fields is Rust data, `literal_words`: Entry "Literal words"; Level 600-cell-Full
  signed generators; Effect pure; Backing library code, `words.power` over `concat` and `inverse`; Notes: construction
  alone checks no state, and a menu word where `power` takes a `Word` is the checker's
  `LB1:input.incompatible_alphabet` (LP2's pull request 210). A test compares the Effect field with the effects the
  checker infers for `power`.
- Exact completion: the source [`completion.6cs`](c600-library/src/completion.6cs) is the module `completion` with one
  exported function, `exact_completion(pieces: List<Nat>, home_slots: List<Nat>) -> Bool`, effect `snapshot`. The
  host names the candidate pieces and their home slots, in the candidates' order. The entry reads KN2.4's `positions`
  row for the pieces and its `labels` row at the named home slots, each with `read_snapshot` and its parameter as
  given, and answers `true` exactly when the list is not empty, the named home slots are the candidates' own and every
  candidate's stickers sit in their own home slots: `positions` answers each candidate's `slots` equal to its `home`
  slots, and `labels` answers each home slot's label as that slot. A label is the home slot of the sticker a slot
  holds, so both rows give identity facts and the entry reads no colour. The built-in column admits the entry with the
  read rows `positions` and `labels`; it reads state and prepares no preview. Its catalogue row,
  `residuals_and_exact_completion`, has Level "Full labels and positions" and Effect `snapshot`, and its Notes say
  that empty candidate lists, matching colours or a buffer whose occupant is not its home piece never establish
  completion.
- Exact linear algebra over F2 and F5 (pull request 274): the source [`linalg.6cs`](c600-library/src/linalg.6cs) is the
  module `linalg` with four pure functions over the IR draft's F2 and F5 (LP2's pull request 222):
  `rank_f2(rows: List<List<Int>>) -> Nat` and `rank_f5`, the rank of the rows in the field, and `solve_f2(rows, rhs)`
  and `solve_f5`, which answer the rank, whether the system with the coefficient rows `rows` and the right-hand side
  `rhs` is consistent and, when it is, one solution as residues (Gauss-Jordan elimination with every free unknown 0).
  Each entry is reduced into the field, so negative entries and entries of any size are allowed. The checker infers no
  effect, capability or state read, so a rank or a solution says something about the supplied equations only, never
  about a puzzle state or its reachability. Its catalogue row is `exact_linear_algebra`.
- Recipes and reports (pull request 294): the source [`recipes.6cs`](c600-library/src/recipes.6cs) is the module
  `recipes`. A recipe crosses the entries as a list of step records in 0.4's order: a word step carries its letters, a
  star step its orbit, node and sign. Seven entries: `word_step` and `star_step` build one step record each;
  `star_word(setup, relocation, seed)` writes out the inverse setup, the inverse relocation, the seed, the relocation
  and the setup, the star of sign 1 as a literal word; `effect_report(recipe)` answers KN2.4's `effect` of the whole
  recipe (the full source-to-destination map, the moved stickers and pieces, the support per orbit, the digests of the
  net map and of the literal expansion, and the expansion's length); `orbit_report(recipe, orbits)` answers the full
  support split into the selected orbits' rows and the others (LB2.2); `frame_image` and `frame_maps` image an ordered
  slot tuple through the recipe's map. The three constructors are pure; the four others have the effect `snapshot` and
  read the `effect` row and no other, so they read state and prepare no preview. Its catalogue rows are
  `star_and_recipe_construction` and `collateral_and_support`.
- Net and Strict protection (pull request 300): the source [`protection.6cs`](c600-library/src/protection.6cs) is the
  module `protection`. Three captures read the head at what the solver names: `position_lock(slots)` reads `labels`, and
  `identity_lock(pieces)` and `block_lock(pieces)` read `positions`. A lock is a list of exact (slot, label) pairs at
  the level `labels`. Three checks read the `protection` row at a named lock and recipe: `net_check`, `strict_check` and
  `strict_check_within(limit)`. LB2.3's report carries its input (the lock, and the expansion's digest and length), its
  level, its mode and its coverage (`checked` against `required`: 1 for Net, primitives plus 1 for Strict), the
  conflicts in the lock's order, Strict's first refused prefix and a verdict of `fails`, `clear` or `unknown`. `unknown`
  carries `LB2:protection.incomplete_coverage` and is never safe. Its catalogue row is `net_and_strict_protection`.
- Guarded setups and buffers (pull request 309): the source [`setups.6cs`](c600-library/src/setups.6cs) is the module
  `guarded_setups` with one entry, `setups_for(goal: record { orbit: Nat, position: Nat })`, effect `snapshot`. It reads
  the `setups` row at the goal as given and returns the answer as read, so it reads state, prepares no preview and
  runs only under the built-in column's setup-search exception. The row answers only for the target the solver names,
  lists its certified setups in a stable order with no rank, keeps the orbit's buffers fixed in every setup and executes
  nothing; applying a setup stays the solver's action. Its catalogue row is `guarded_setups_and_buffers`.
- Exact Q(√5) signs (pull request 325): the source [`quadratic.6cs`](c600-library/src/quadratic.6cs) is the module
  `quadratic` with two pure functions over the IR draft's `Q5`, the numbers (a + b√5) / d: `sign_q5(x)`, the exact sign
  of `x` as -1, 0 or 1, and `compare_q5(x, y)`, the sign of `x - y`. Both decide with the IR draft's exact comparison
  (`exact_less`, KN3.1), never with an approximation, and say something about the supplied numbers only. Its catalogue
  row is `exact_q5_signs`, a second part of the row "Exact linear algebra".
- Quaternion factors and tree outputs (pull request 347): the source [`trees.6cs`](c600-library/src/trees.6cs) is the
  module `trees` with seven pure functions over `Q5`, with the meaning of `research/jumbling/theory/trees.py`:
  `factors(m)`, a pose matrix's left and right quaternion factors exactly as trees.py extracts them (not up to scale)
  and whether they reconstruct it; `reconstructs(m, l, r)`; the 2-adic valuation `v2`; `kval`, `dist` and `branch` on
  the 5-regular tree of the icosian order at 2; and `pose_tree(m)`, the branches and distances of `m`'s factors. Its
  catalogue row is `quaternion_and_icosian_algebra`.
- Word equality keys (pull request 350): the source [`word_keys.6cs`](c600-library/src/word_keys.6cs) is the module
  `word_keys` with two pure functions on 600-cell-Full words: `key(w)`, LB1.10's draft `trace-key/1` under the relation
  the run pins, `full-disjoint-support/1` (two letters are independent when their generators move disjoint sets of
  slots), and `same_word(x, y)`, true exactly when the two keys are equal. The owner's answer of 17 November 2026 gives
  C5's word keys `trace-key/1`, so the entry keys with the IR draft's `trace_key`, not with its free reduction
  `word_key`, which merges an inverse pair with the empty word. A key has its own type and is never letters; it keeps
  every letter and is sound, not complete: equal keys give the same end digest at every state. Its catalogue row is
  `word_equality_keys`.
- Named macro analysis and comparison (pull request 353): the source [`macros.6cs`](c600-library/src/macros.6cs) is the
  module `macros` with two entries, effect `snapshot`, over a name the caller gives and a recipe in the step records of
  `recipes`. `macro_report(name, recipe)` reads KN2.4's `effect` and `replay` rows at the recipe and answers the macro's
  whole effect, the session's protected orbits at the head, Net's conflicts (the protected orbits the net map moves) and
  Strict's first refused prefix with the orbits in conflict there. `macro_comparison(first, first_recipe, second,
  second_recipe)` reads `effect` at both recipes and answers the support orbits both net maps move and those only one
  moves, and whether the net maps and the literal expansions are equal. The entries report the caller's macros and
  choose none: no answer ranks, selects, previews or applies a macro. Each recipe is a parameter of its own, because the
  built-in column admits a read only at a parameter as given (LB3.5). Its catalogue row is
  `named_macro_analysis_and_comparison`.
- Seeds and their stage order (pull request 357): the source [`seeds.6cs`](c600-library/src/seeds.6cs) is the module
  `seeds` with two entries. `seed_report(orbit, seed)`, effect `snapshot`, reads `effect` at the seed the caller names
  and answers its whole net effect, what it moves in the target orbit the caller names, and its collateral, every other
  orbit its net map moves, from the kernel's full map, so an empty collateral is established by the map, never by
  omission. `stage_order(seeds)`, pure, orders the caller's rows of collateral and orbit so that every orbit comes
  before each orbit its seed moves: at each step the first ready row in the caller's order, so no orbit's ID or rank
  decides, and rows the edges hold in a cycle come back in `cycle`, none dropped. The entries choose no seed, orbit or
  target. Its catalogue rows are `seeds_and_full_seed_analysis` and `collateral_stage_order`.
- Orbit catalogue, progress per orbit and residuals (pull request 363): the source
  [`orbits.6cs`](c600-library/src/orbits.6cs) is the module `orbits` with three entries. `orbit_catalogue(named)`,
  effect `snapshot`, reads the `orbits` row at the orbits the caller names, -1 (the fixed cell centres) to 34, and
  answers each orbit's pieces by home position, model data the same at every head. `orbit_progress(orbit, positions)`,
  pure, counts among the caller's `positions` records of the orbit the pieces with every sticker home (`solved`), at
  Home with a sticker elsewhere (`orientation_wrong`) and not at Home (`position_wrong`), and counts the records of
  other orbits apart (`others`), so none is dropped unseen. `orbit_residuals(members, buffer_a, buffer_b, positions)`,
  pure, takes the orbit's catalogue row, its buffers A and B as the `setups` row answers them and the caller's
  `positions` records, and answers the misplaced and misoriented non-buffer pieces, each buffer's occupant and whether
  it is solved, the pieces whose stickers are split over positions (`unknown`), whether the records name exactly the
  orbit's pieces in the catalogue's order (`complete`), and the exact solved status, which needs every member named with
  every sticker in its home slot. The row supplies membership only; the progress and the residuals take the positions as
  a parameter (chapter 03), which a host reads at what the solver names (LB3.5). The entries choose no orbit, target or
  buffer and rank nothing. Its catalogue rows are `orbit_catalogue_and_progress` and `residuals_per_orbit`.
- Tests (`c600-library/tests/`), each naming its part; CT-LB1-01 runs whole in two of them, `ct_lb1_01.rs` and
  `ct_lb1_01_trees.rs`, and no other conformance ID runs whole here:
  - `ct_lb1_03.rs`: CT-LB1-03's 600-cell-Full half. The language part: the source parses, keeps its canonical layout,
    lowers to one module that requests no capability and checks with no diagnostic, and `power` has no effect, reads no
    state, prepares no preview and names no proposal target; `power` and a client that imports it by digest build the
    expected letters on `c600-run` with no host (`power(word[+1, -2], -2)` is `[+2, -1, +2, -1]`, and
    `commutator(word[+1], word[+1])` is `[+1, +1, -1, -1]`). The application part: `Kernel::replay`, fed a
    library-built word in chunks, reports the first prefix that moves a protected orbit as soon as that prefix is in
    (the first prefix on the shipped model, where every generator moves every orbit), clears every prefix when no orbit
    is protected, and ends where `Kernel::derive` ends. The inverse part: a power and a commutator are admitted from
    solved and from a scrambled state, and their inverse, applied from the end, retraces the path prefix by prefix
    back to the start.
  - `ct_lb2_02.rs`: CT-LB2-02's Strict half at the kernel, on library-built words with identity net effect whose
    intermediate prefixes move a protected orbit: Net derives them and ends at the base; Strict refuses them at the
    first moving prefix with one certificate per protected orbit that prefix moves; a replay gives no verdict before
    `finish()`, and a replay finished early judges only the letters pushed. The Net and Strict halves run through the
    tool's route on the command layer in `c600-tools/tests/net_oversize.rs`, and the whole run, with the captured lock,
    is in `c600-tools/tests/ct_lb2_02.rs`.
  - `completion.rs`: CT-LB2-04's language part. The exact-completion source parses, keeps its canonical layout, lowers
    to one module that requests no capability and checks with no diagnostic; the entry has the effect `snapshot` and
    reads state through snapshot reads of `positions` and `labels` and no other row, so it prepares no preview; the
    catalogue row's Effect field is the checker's.
  - `ct_lb1_01.rs`: CT-LB1-01's F2, F5 and Q(√5) sign parts. Against the pinned research references: the 36-macro
    correction library's ν rows have rank 34 over F2 and 2 over F5, and every row is 0 on the nine coordinates no
    correction changes; the senior review's F5 cycle criterion holds for every nonzero ratio on cycles of 2 and 3 edge
    cells. Fixed F2 and F5 systems with negative entries agree with a test-local elimination, and a solve reads no
    state, so it asserts no puzzle reachability. The research pins no Q(√5) sign fixture (LX-5's planned ones were not
    written), so `sign_q5` and `compare_q5` are checked against the kernel's exact sign of a + b√5
    (`c600_kernel::exact::sign5`, KN3.1) on the convergents of √5, where a² - 5b² = ±1 and the sign is hardest to
    decide, up to 128-bit coefficients.
  - `ct_lb1_01_trees.rs`: CT-LB1-01's quaternion factor and tree parts: every answer of `trees` equals trees.py's exact
    answer in `tests/fixtures/trees-reference.json`, factors coefficient for coefficient, and trees.py and its
    `exact.py` are still the pinned bytes.
  - `recipes.rs`: CT-LB2-01's source part. The recipes source checks with no diagnostic and keeps its canonical layout;
    the three constructors are pure and build step records and literal star words with no host; the four other entries
    read the `effect` row and no other; the two catalogue rows split the exports between them, with the checker's
    effects.
  - `protection.rs`: CT-LB2-02's source part. The protection source checks with no diagnostic and keeps its canonical
    layout; each capture and each check reads its own rows (`labels`, `positions` or `protection`) and no other; the
    catalogue row's effect is the checker's.
  - `setups.rs`: CT-BND-18's source part. The setups source checks with no diagnostic and keeps its canonical layout;
    the entry reads `setups` and no other row; the catalogue row's effect is the checker's.
  - `macros.rs` and `seeds.rs`: the source parts of the named macro analysis and of the seeds. Each source checks with
    no diagnostic and keeps its canonical layout, each entry reads its rows and no other (`stage_order` none), and the
    catalogue rows' effects are the checker's; `seeds.rs` also runs the pure stage order with no host, a cycle included.
  - `orbits.rs`: the source part of the orbit catalogue, progress and residuals. The source checks with no diagnostic
    and keeps its canonical layout; `orbit_catalogue` reads `orbits` and no other row, and the other two are pure; the
    two catalogue rows split the exports, with the checker's effects. With no host, the progress and the residuals
    answer hand-made records as defined, and an empty, partial, longer, reordered or repeated list, or one with another
    orbit's record, is neither complete nor solved.
- CT-LB2-04's whole run is in `c600-tools/tests/exact_completion.rs`, since the library crate is a dev-dependency of the
  tools crate: whole, on 600-cell-Full, through `Boundary` and the tool's host bridge onto a command layer in the
  solving profile, author `built_in`. The solved state passes, for one piece of each of the 35 orbits and of the fixed
  cell centres, together and each alone. These do not pass: an empty list; a piece in its home position but turned,
  after a fixed scramble; two pieces of one orbit whose home slots lie in the same cells, exchanged, so that every slot
  keeps its colour; and a buffer holding another piece of its orbit, a residual that a residual check would permit. The
  exchanged and residual states are label arrays built from solved with `Kernel::state_from_labels`. Each case checks
  the identity facts the kernel gives (positions, slots against home slots, home labels), not colours. The built-in
  column refuses a variant that reads `labels` at home slots taken from the `positions` answer (LB3.5), and the entry
  answers `false` when the named home slots are not the candidates' own, in order.
- CT-LB2-01's whole run is in `c600-tools/tests/ct_lb2_01.rs` (pull request 294), on the same host, against the
  differential oracle's cases O01 to O07.
- CT-LB2-02's whole run is in `c600-tools/tests/ct_lb2_02.rs` (pull request 300), on the same host.
- CT-BND-18's whole run is in `c600-tools/tests/ct_bnd_18.rs` (pull request 309), on the same host.
- The named macro analysis runs whole in `c600-tools/tests/macros.rs` (pull request 353) and the seeds in
  `c600-tools/tests/seeds.rs` (pull request 357), on the same host; the word keys run on the pinned model's relation in
  `c600-tools/tests/ct_lb1_02.rs`, CT-LB1-02's 600-cell-Full half (pull request 350).
- The orbit catalogue, progress per orbit and residuals run whole in `c600-tools/tests/orbits.rs` (pull request 363), on
  the same host.
- Drafts, in the crate documentation under "Drafts": the exponent is an `Int`, and a negative power is the inverse
  written out; `power` recurses through `exact_add`, because the IR draft has no subtraction or negation and had no loop
  when `power` was written (loops came with LP2's pull request 234); a power is a flat word, not a compressed one; the
  catalogue rows are Rust data, because the IR draft and the syntax carry no module manifest; the library builds no
  incompatible-alphabet refusal of its own. Exact completion answers a `Bool`, `false` for an empty list, for which
  chapter 13 registers no code; completion is identity from both rows, with no buffer and no residual; the entry takes
  the candidates' home slots as named, because the built-in column reads only at what the solver named (LB3.5), and
  checks them against the `positions` answer; `repeat` and a recursive `flatten` stand in for a list map, and
  `flatten`'s cost grows with the square of the number of candidates (deferred). The `labels` row's argument, answer,
  codes and tariff are `c600-tools`' drafts, and so are those of the `effect`, `protection`, `replay`, `setups`,
  `orbits`, `structure.compare` and `filter.preview` rows. The drafts of exact linear algebra (two copies of one
  elimination, matrices as `List<List<Int>>` and solutions as residues), of recipes and reports, and of the later
  entries (Q(√5) signs, quaternions, word keys, macros, seeds and orbits) are in the crate documentation too.
- The jumbling library entries, on the IR draft's menu words, are C8's (`c600_jumble_tool::library`). Not built: library
  entries over kernel rows other than exact completion, recipes and reports, Net and Strict protection, guarded setups,
  named macros, seeds and the orbit catalogue: the IR draft passes a run's input as a query's argument since LP2's
  `read_snapshot<T>("q", x)`, and the tool's bridge answers such a read for `positions` (LP2's pull request 226),
  `labels`, `effect`, `protection`, `replay`, `orbits` (pull request 358), with the setup table `setups`, and the
  representations tool's `structure.compare` and `filter.preview` (C6) only; in LB2's residual half, frames relative to
  a reference the solver names; the rest of LB2's catalogue over `effect` (blocks and work sheets, and their residuals);
  in the seed rows, the target action as cycles with its exact frames and KN6's certificates; a colour count in the
  progress per orbit; a captured lock in a macro report; the cosets γK⁺, and the rest of "Exact linear algebra" (null
  spaces, and ranks and solving over Z, Q and Q(√5)); keys of jumbling menu words in this crate (LG2.18's key on the
  pinned menu matrices is C8's); the `labels` row's refusal with Jumble on, which is built and not run, because a
  jumbled layer needs `c600-jumble`; the other entries of LB1.8's table.
- Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg was not run.

## Status of LP3, C9 bridge: the display host over the S-A2 core

- `c600-render` implements `c600-display`'s `Backend` over ABI 5 of the S-A2 renderer core
  (`renderer/core/include/sa2_interop.h`) through `CoreAbi`, a safe call interface, and forbids unsafe code.
- The foreign half is `c600-render-ffi` in `renderer/bridge`, a crate with a workspace and a lockfile of its own,
  because CT-RPI-01 keeps every member of this workspace free of unsafe code and foreign declarations. There
  `CoreFunctions` is a table of the core's 17 C functions, and `DllCore` implements `CoreAbi` over it: it checks every
  array length and sets every row's size before it calls one, and refuses a null context or another ABI version. The
  bridge loads no library: the shell links `renderer/bridge`, loads `sa2_interop.dll` and hands over the context and
  the table, because only `c600-platform` calls an operating system's own API (CT-RPI-20) and it has no loader.
- The shell's native host library, `c600-shell-host` in `renderer/bridge/host`, is a member of that workspace, and so is
  LX-4's editor library, `c600-shell-lx4` in `renderer/bridge/lx4` (C9 U8, measurement tooling, loaded only with
  `--lx4`). It is step S2 of `renderer/shell/README.md`, a draft on route B, accepted for now by the owner on 18 Nov
  2026, pending three owner-machine checks. It runs `c600-display`'s host over `RenderBackend<DllCore>` behind a C
  interface (`m6h_*`) that the shell's C# side calls, and it loads no library either.
- The mapping: the core's describe report becomes the support report, with the limits the caller declares. J01 and J02
  are passed through as the core describes them: the bridge adopts a jumbling bundle's pose arrays over the home labels,
  and a posed plan draws every slot with no slot pickable; an instance of a posed primitive (J13) stays unsupported. F20
  (atomic publication and latest-wins replacement) is not a drawing row and the core does not name it; its backend
  operations are ABI 4's prepare, poll, activate, cancel and retire, and its replacement rules are the host's (RM.17 to
  RM.22), so the bridge reports it supported unless the core names it. A preparation carries at most one retained
  primitive, its labels a permutation and its instances on stickers or slots; anything else is refused by the feature it
  needs. Activation sends the publication stamp's digest. A frame needs a camera matrix for its view and an active set.
  On a `sa2_attach` context (the Godot-texture route) `read_presented` and `recover` answer renderer unavailable (F23;
  F22 and RPI.46), so the host's presented latch stays clear. On a `sa2_create` context with a surface (ABI 5, route B,
  accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks) frames name `SA2_TARGET_SURFACE`,
  `read_presented` returns the frames the core confirmed under the exact rule (the bridge keeps each one's display time,
  the SyncQPCTime of the confirming statistics row, for `take_display_times`), and `recover` rebuilds a removed device:
  the bridge then forgets every resource set of the old epoch, so an installed publication draws again only after it is
  prepared and activated on the new device (RM.23). The `Backend` trait's adoption, frame reports and picks map onto
  the bridge's own calls of the same names, which stay public: engine adoption; frame reports in batches of 64, each
  with its frame's stamp; pick requests and polls by the core's frame ID (a pick names the binding field that went
  stale); and the core's counters, which have no trait operation. Through the trait, a bundle at the revision the core
  already holds (the whole revision reference) is adopted without a second copy, since the shell adopts into the core
  first, and any other reaches the core, which refuses a revision that does not increase; frame reports are empty,
  since RM.41's reports name frames that stopped a user stage and the product path runs none; and a pick on a frame
  older than every frame the bridge keeps is stale at `sequence`, one on a stamp it never submitted is invalid, and so
  is a point outside the frame's viewport; a frame the core certainly no longer retains (64 newer frames reported, or
  an earlier device epoch) is stale at `sequence` before the core is asked, and the core's own refusal (for a frame it
  dropped otherwise, among others) is a core failure.
- User stages (C9 U5a; drafts until the IR freeze): the module `stage` takes a stage that `c600-check` admitted
  (`CheckedStage`) and gives its parameter binding (binding/1), deterministic HLSL under `m600-stage-hlsl/1`, the
  binary32 CPU reference evaluator (RPI.74) and a strict reader of a limits file. The bin `c600-stage-hlsl` writes
  `<name>.hlsl` and `<name>.json` for one `.6cs` stage. `renderer/core` compiles the text with DXC and runs RPI.72's
  chain (ABI 5, additive); the host attaches no user stage yet. Tests: `stage_hlsl.rs` (golden text, the reference
  against hand evaluation, the binding's refusals, the gate pair in `renderer/acceptance/stages` with its overdraw 108,
  the limits file).
- Tests (`c600-render/tests/`): `abi_layout.rs` reads the header and checks every row's offsets and size and every
  constant against the Rust side; `bridge.rs` runs the host over the fake core: adoption, support, publication and
  install, frames and their reports, picks and their staleness after a view reopens or a set is retired,
  replacement, and device loss; on route B also confirmed presentations and their display times, the
  presented-preview latch, a contradicting presented row, recovery, a pick on a frame of the old device epoch, and a
  publication drawn again on the new device.
  The scenarios live in `tests/support`, which `renderer/bridge`'s tests include and run again through the C function
  table over a C shim; there `abi_functions.rs` checks every function's signature against the header and `ffi.rs`
  what `DllCore` refuses before a call and what it reads back. `trait_ops.rs` drives the trait's adoption, frame
  reports and picks through the host: adoption in the shell's order and alone, another image at the same state
  counter, a pick requested, polled and resolved, a point outside the frame's viewport, and frames the core or the
  bridge no longer keeps.
  `tests/test_render_ffi.py` runs that crate's tests, since `tests/test_language_rust.py` runs this workspace only.
- Not run: no run against the built core library and no GPU run, so nothing here is Windows/DirectX, input or
  performance evidence. Not built: a check of the preparation's base revision by the core.
- Evidence kind: source and fixture. The checks ran on Windows only; the Linux leg was not run.

## Status of LP3, C9 measurement: LX-4's module set and edit script

- `c600-lx4` generates RTE.48's fixtures from whole declarations of the three LP3 tools (PHS.22), renamed, in an
  order seeded by 2026111801, with `#` comment padding to the exact length: S, M and L are single modules of 200,
  1,000 and 5,000 LF-terminated lines; W is 50 modules of 400 lines, an acyclic synthetic import graph by digest
  (module k imports one to three earlier modules; W01 to W09 import W00), not a real LP3 graph. Every module exports
  `exported() -> Nat`, and a W module's export calls its imports' exports.
- The edit script (seed 2026111802) holds RTE.51 step 2's four kinds at S, M, L and W00: inside a body, exported
  signature (`Nat` to `Text`), completion trigger (an unfinished `exact_` at the end, with its cursor) and syntax
  break. Each edit starts from the original bytes and refuses a stale baseline, a differing preimage or result.
- Before writing, the module set and every edit go through `c600_syntax::parse` and `lower` and `c600_check::check`:
  every module is clean; inside-body edits stay clean; the signature edit is clean at S, M and L and, at W00 with all
  49 later modules re-pinned, refused only in `exported` of W00's 13 direct dependents; the other two kinds stop at
  syntax. A disagreement refuses the whole output (LX-4's immediate kill criterion).
- `c600-lx4-fixtures --out <new folder>` writes `S.6cs`, `M.6cs`, `L.6cs`, `W/W00.6cs` to `W/W49.6cs`,
  `MANIFEST.json` (`magic600-c9-lx4-fixtures-v1`: the tools and every module by digest) and `EDITS.json`
  (`magic600-c9-lx4-edits-v1`). It never writes over a folder that holds anything.
- DRAFT: the set follows the tools as they stand; RTE.51 step 1 freezes it with the candidate and its build identity.
  The modules hold the tools' declarations, not their run-time views, and the LX-4 harness (Jumble on and off, the
  owner-machine runs, the frame matching) is not built. Evidence kind: source and fixture; nothing here is timing.

## Status of LP3, C9 measurement: LX-2's metered and forced-kill half

- `c600-lx2-faults --runtime <c600-runtime binary> [--injection <c600-lx2-child binary>] [--sessions <new folder>]
  [--assets <folder>] [--out <record.json>]` runs five trials on `c600-runtime`'s parent API against the child binary
  it is given, so the decisive runs use the product build's runtime unchanged, and with `--injection` three native
  trials on the injection device. Build and run both bins with the feature `device`. Every job first proposes
  SCENARIO.json's runtime word `[7, -11, 1200]`, which the host keeps as the pending preview.
- `control` completes; `loop_metered` (a `repeat` loop with no kernel call, 100,000 steps) ends `KN7:budget.steps`;
  `recursion_metered` (endless recursion, 1 MiB) ends `KN7:budget.memory`, since the meter charges a frame as memory
  and has no recursion dimension; `alloc_metered` (a list grown with `push`, 64 KiB; each `push` is priced as a new
  list and, since LP2's pull request 364, extends the list in place) ends `KN7:budget.memory`; `forced_kill` kills the
  child's process inside the proposal's perform (the barrier) and must end in a typed `Lost` fault
  (`KN7:resource_limit.runtime_lost`). After every trial the pending preview is still the runtime word and the same
  runtime completes the control again, in a fresh child after a fault and in the same child otherwise. The metered
  trials and the control also run in process, and the two executors agree.
- With `--sessions`, each trial also runs with a session open (RTE.42 conditions 1 and 2; RTE.43): KN5's command
  layer over M6's journal on a file-system disk in its own fresh subfolder, with the shell and the headless display.
  The solver commits `[1, -2]` and invokes the trial's tool row from the shell; the trial's job runs as that command's
  program job in the child, through the product's bridge and boundary, so its proposal is the layer's pending preview
  before the fault; the solver then commits that preview and `[3]`. The job is the process host's job for the tool
  (`c600_tools::process::job`: the solving profile and the tool's class rows, so that the child admits the entry in
  the built-in column) with the trial's limits and no summary rows; `tests/session.rs` checks it against the host's. A
  trial agrees when it ends with its typed outcome, the solver's command that ran it ends as that outcome says, every
  solver command after it is accepted, its journal records and state hash equal the control's, and a reopen replays
  the same state hash. The journal is read before the reopen, which journals a fresh random epoch of its own.
- The injection device of option (a) (top integrator, 18 November 2026) is the bin `c600-lx2-child`: product source
  (`c600_runtime::child::main`, or with this package's feature `device` `c600_runtime::child::main_with` on
  c600-runtime's fault hook, LP3's feature `lx2-faults`) under a `#[global_allocator]` that wraps the system allocator
  and refuses an allocation once the live heap would pass the byte count in `C600_LX2_CEILING`. Its unsafe code is in
  that bin only; c600-runtime keeps no allocator of its own. Build it for the decisive run with the product's profile
  and the feature `device`: `cargo build --locked --offline --release --manifest-path rust/Cargo.toml -p c600-lx2
  --features device --bin c600-lx2-child`. It is never shipped: `tests/device.rs` checks, through `cargo metadata`,
  that no package of this workspace or of `renderer/bridge` other than c600-lx2 depends on c600-lx2, that only
  c600-lx2's feature `device`, which no default turns on, turns the fault hook on, that the runtime child built as the
  product builds it has no hook, and that nothing under `c600-runtime/src` names a global allocator.
- `alloc_ceiling`, a native trial, runs only on the device, with the ceiling at 32 MiB set in `Config.env` for the
  trial's child and its recovery child, and the meter at 1 GiB as a safety net: a list doubled with `concat` passes the
  ceiling, the standard library reports the failed allocation and aborts the child, and the parent must report a typed
  `Lost` fault (`KN7:resource_limit.runtime_lost`) whose standard error shows "memory allocation of"; a loss without it
  does not count. Then a fresh child completes the control, as after the forced kill. With `--sessions` it also runs
  with a session open and must agree with the control.
- `recursion_native` and `loop_native`, the other native trials, need the device built with the feature `device`,
  which refuses them otherwise. Their job is the control's. The device arms its native fault (`C600_LX2_FAULT`) for
  the trial's child, and the hook fires after two answers, the proposal's tariff and perform, so after the pending
  preview exists and before the job's result. The device writes its line to standard error and then either recurses
  in native code until the stack overflows, where the parent must report a typed `Lost` fault
  (`KN7:resource_limit.runtime_lost`) whose standard error shows the overflow, or spins in native code without a
  kernel call until the parent's watchdog, the run's wall-clock safety stop at 3 s (KN7.10), ends the child with a
  typed `Deadline` fault (`KN7:resource_limit.safety_stop`). The fresh child that recovers runs unarmed. With
  `--sessions` both also run with a session open and must agree with the control.
- The record (`magic600-c9-lx2-faults-v1`) names each child by file name and SHA-256 only, never by path. Each row
  names its child (`product` or `injection device`); `injection_device` is null without `--injection`, and otherwise
  carries the device's file name and SHA-256, "product source + injection device", the ceiling's variable, the
  ceiling, the meter limit, the fault variable and the answers after which a native fault fires. The limits name the
  deadline and the watchdog. Each session row names the journal by row count and digest, never the folder.
- Not here: the barrier's release acknowledgement, which needs the shell's frame loop on the owner's machine.
  Conditions 3 and 4 (the renderer gate, LX-4's echo bar and E2 commits during fault windows) are built in the shell,
  not in this crate: `renderer/shell/project/Lx2/Lx2Module.cs` and `tools/perf/c9/lx2.py` run and judge turn windows
  and, for the metered faults, limit windows (C9 U7), and a decisive run needs the owner's machine. A limit window ends
  at the meter's budget code or at the host's 60 s view deadline, `KN7:resource_limit.safety_stop`, whichever comes
  first. `c600-lx2/examples/view_limits_probe.rs` measures how far a metered fault runs under the product's view
  limits. Evidence kind: source and synthetic; nothing is timing.

## Status of LP4-1: the editor services

`c600-editor` holds EV1's editor services (chapter 04, EV1.1 to EV1.12) as a library that LP4-3's C ABI host library
(`renderer/bridge/editor`, not built yet) wraps. Its API is plain owned data: strings, vectors, enums and byte offsets,
with no handle kept between calls and no callback. Like the IR, it is a draft until the language's interfaces freeze.

- No user program runs (EV1.9). The crate forbids unsafe code and depends on `c600-syntax`, `c600-check`, `c600-ir`
  and `c600-digest` only; `tests/dependencies.rs` follows the manifests and fails on any path to `c600-run` or
  `c600-command`. A shell links it as a library; program execution stays outside the UI thread.
- Documents and ranges (EV1.1, EV1.10). `Document::open(bytes, units)`, or `open_declared(bytes, "utf8" | "utf16" |
  "scalar")`, names immutable bytes by their SHA-256 (`digest()`) and keeps the units the host declared. Every service
  takes the digest its request was built against, and positions in the declared units; another digest is
  `EV1:stale.editor_source`. `SourceRange { digest, start, end }` is H7's: UTF-8 byte offsets on scalar boundaries,
  half-open. `offset`, `range`, `host` and `host_in` convert from and to the declared units without rounding. An
  endpoint inside a scalar or past the end, a reversed range, units other than the declared ones, and bytes that are
  not pinned source (malformed UTF-8, a CR, a byte-order mark) are `EV1:input.source_range_invalid`, each with its
  typed reason; for unpinned bytes the reason carries ERR.10's raw context, the raw digest and the offending byte
  interval, and never a converted position.
- Subjects and contexts. Every response carries a `Subject`: the source digest, whether the bytes are pinned, the
  edition and the module context's digest. A `ModuleContext` holds the edition, the session's model identity and the
  identities the draft supports (KN0.1's five fields), the evidence obligations, and the dependency sources by digest
  with the identities each supports; `ModuleContext::digest` is a draft canonical text,
  `c600-editor-context-draft-1`.
- `parse(document, edition)` (EV1.2): the parsed file with its recovery holes, what was read of each broken
  declaration (kind, `export`, name, and every binding with the byte span it is visible in), comments and whitespace
  as trivia, an outline in source order (imports, declarations, holes, comments) and the syntax diagnostics. An
  encoding fault gives ERR.10's diagnostic with no span.
- `complete(document, digest, position, context)` (EV1.3): the completion context at the position (comment, text,
  literal, declaration, signature, new name, type, effects, capabilities, a dependency's members, a field or an
  expression), the range the prefix replaces, and the candidates, sorted by label only, each with its kind, type,
  effects, declaring identity (alias, source digest and declaration) and documentation reference. Locals come from the
  recovered scope, so an incomplete declaration still completes; members come from the pinned dependency. A missing
  dependency or unknown field types are listed in `missing`. A context whose session identity the draft does not
  support, or a dependency that supports another identity, is refused with `KN1:input.identity_mismatch`. Nothing
  ranks candidates or proposes a move (the human-solve boundary).
- `diagnose(document, digest, context)` (EV1.4): syntax diagnostics; `LG5:input.module_dependency_missing` at an
  import whose bytes the context does not hold; `KN1:input.identity_mismatch` at the module header or at the import,
  naming the first differing field; `LG4:input.ir_rejected` for a lowering refusal; the checker's findings forwarded
  with their own codes and typed context at the range the source map gives, so LG2.22's direct and helper condition
  refusals and its scalar `exact_required` case pass through unchanged; and `EV1:evidence.obligation_unmet`. The
  `Verdict` is `Checked(CheckedCandidate)` only for a complete draft that lowers and checks with no finding.
  `CheckedCandidate` names the source, module, edition, closure digests, IR version, execution contract and context
  digest, the data that EV1.11's activation names; it grants nothing.
- `hover` and `definition` (EV1.5, EV1.6) resolve a name in the draft and its pinned dependencies: the declared type
  or signature, declared effects, the required capabilities, exactness, the module's static check, the checker's
  inferred effects and capabilities with capability separation's view, the permission `DecidedAtInvocation`, and the
  defining range under its own source's digest. Each field it cannot resolve says why (`Unresolved`); a missing
  dependency is never replaced by another version, and no check is invented.
- `rename(document, digest, position, proposed, context)` (EV1.6) proposes edits, each bound to its source's digest,
  the draft's first. Renaming a dependency's exported declaration edits that dependency's bytes, named by its old
  digest, and re-pins each importer's digest literal up the closure in dependency order, so no import keeps naming
  bytes that changed. The edited closure is checked again, and its diagnostics and verdict are returned: a clash or a
  malformed name shows there, and the proposed name is not judged first. `apply_edits(document, edits)` applies one
  source's edits; edits for other bytes are `EV1:stale.editor_source`, overlapping edits and invalid ranges
  `EV1:input.source_range_invalid`.
- Envelopes (EV1.12, chapter 13). Every diagnostic and refusal is an `Envelope`: the code, the form, the service that
  raised it (the event), a typed `Reason`, a typed `NextAction`, the range when it has one, and a summary for display
  only. `category`, `recoverable` and `retry` follow ERR.8 to ERR.15. The codes are constants in `c600_editor::code`.
- Additions to `c600-syntax`, additive only, so its other tests run unchanged: `ParsedFile.trivia` (comments and
  whitespace), `ParsedFile.recovered` (`ast::RecoveredDecl`), `lexemes`, `pinned` (the byte check alone),
  `type_text`, and the name lists lowering reads (`base_types`, `type_constructors`, `precisions`, `effect_names`,
  `capability_names`, `orders`) (`c600-syntax/tests/authoring.rs`).
- Tests. CT-EV1-01, CT-EV1-02, CT-EV1-03, CT-EV1-05 and CT-EV1-07 run whole (`c600-editor/tests/ct_ev1_0N.rs`, each
  test named by its ID); `tests/completion.rs` covers completion contexts and `tests/dependencies.rs` EV1.9's
  dependency part.
- DRAFT: the evidence obligation. The IR draft has no claim types, so a declaration the context requires to supply
  evidence meets the obligation when it, or a function it calls, has an exact evidence sink (`use_evidence`).
- Not built, which is no pass (PHS.63):
  - a layout that keeps comments: `format` still drops them;
  - inferred types of locals: the checker reports none, so hover gives a local's declared type or says why not;
  - model and level on hover, and evidence references: the IR draft has no identity tower, levels or claims;
  - field completion beyond the declared record types of parameters;
  - renaming a binding inside a recovery hole: such occurrences stay unchanged and are listed in `skipped`;
  - a missing dependency of a dependency is reported at the draft's import; identities are compared for the draft's
    direct imports only;
  - EV1.11's activation (LP4-2 builds it on `CheckedCandidate`) and the C ABI host library (LP4-3).
- Not run here: CT-EV1-04 and CT-EV1-08 (EV1.7, EV1.8 and EV1.11 need activation), CT-EV1-06 (LX-4's bars, on the
  owner's machine).
- Evidence kind: source and synthetic. A pass is not Windows/DirectX, input, long-session or performance evidence.

## Status of LP4, LP4-2: version separation and module activation

LP4-2 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (chapter 04 EV1.7, EV1.8 and EV1.11; chapter 02
LG5.6).

- `c600-module`'s `Modules` is a host's set of user modules for one session identity. `check(manifest, lock, files,
  entries)` verifies a draft's closure, approves its entries and creates the module's candidate, returning its digest
  (the closure digest) and its `CheckReference`; it never changes the active version, and a refused check, such as a
  broken draft, stores nothing. `activate_module(module, expected_active_digest, candidate_digest, check_reference)`
  is the only switch: under one lock it checks that the reference is that of the candidate's successful check
  (`EV1:input.module_not_checked` otherwise), that every checked entry's grant lies within the host's grant
  (`LG3:capability.not_granted`), and that the active digest is the expected one, none for an initial install
  (`EV1:stale.activation_base`), and only then installs the whole candidate closure; success returns the previous and
  new digests. `bind(module, function)` binds an invocation wholly to the active version under the same lock;
  `bind_pinned(module, function, digest)` is the route that names a digest. A `Binding` holds its version's bytes,
  approvals and loaded program for as long as it lives, runs in process (`Binding::run`, with the version's own source
  map, so a stop's range names the bytes that ran) or through another executor of the closure (`Binding::run_with`,
  over `Ready::run_with`, which keeps the boundary and the separation gate in the caller's process), and its result
  names the version (`BoundRun`).
- `c600-runtime`'s `BoundJob` builds a job wholly from one binding: the version's bytes in the lock's order, the
  approved entry and the approval's profile, grant and declassifiers; `Runtime::run_bound` runs it. The wire is
  unchanged: the child gets the bytes with the run, so a run suspended at a host call resumes on them after another
  version activates.
- `c600-tools`' `modules` is the host's side: `bind_job` binds a user module's overlay-row program job when the host
  takes it, whichever way the solver invoked the row; `bind_view_source` admits a view source only when its module
  digest names the row's module's active version; `run_bound_in_process` and `ProcessHost::run_bound` run a binding on
  either host.
- Activation is a module service: H-08's stored table has no row for it, and none is added. Activation is explicit
  (owner, 23 November 2026, EV1.13 for the stage 2 greybox): a check never activates, and only a call of
  `activate_module` switches.
- Tests: CT-EV1-04 is whole by union of `c600-module/tests/ct_ev1_04.rs` (three broken drafts, each refused with its
  own code while A keeps running and its results and ranges name A; and the set's routes: invocation by name, a route
  naming B's closure or source digest, activation, B's bytes under A's approval) and `c600-tools/tests/ev1_host.rs`
  (the program job from the palette and from a key on both hosts, and a view source, while a draft is broken, B
  unchecked, and B checked and not active), for the routes that exist. CT-EV1-08 is whole by union of
  `c600-module/tests/ct_ev1_08.rs` (the service, the check reference, every refusal leaving A active, the frozen
  vector `old-run-survives-activation` on a run suspended at a host call, a binding outliving the set, and eight
  threads racing one activation), `c600-runtime/tests/ct_ev1_08_process.rs` (the same run suspended in the child, and
  bound jobs racing activation) and `c600-tools/tests/ev1_host.rs` (a program job suspended on either host).
- Drafts, which freeze with the module formats and the editor protocol (LG5.10, LQ10): the check record
  `c600-check-reference-draft` version 1, whose SHA-256 is the check reference (no FP.4 row, never stored); the
  candidate's digest is the closure digest; one candidate per module, which a new successful check replaces; a check
  names one or more entries of the root module; the host's grant in the set, checked at activation and at each bind;
  a module with no active version, or an entry its check did not cover, is `EV1:input.module_not_checked`; on a route
  that names a digest, a digest no check produced is `EV1:input.module_not_checked` and a checked digest that is not
  active is `EV1:stale.activation_base`, whose continuation is `after_refresh`. Chapter 13 names `activate_module` as
  both codes' origin, so this reading on a route is the integrator's to confirm. The list is in the documentation of
  `c600_module::activate`, `c600_runtime::bound` and `c600_tools::modules`.
- Crate edges: `c600-runtime` and `c600-tools` depend on `c600-module`, and `c600-tools` has `c600-input` as a
  dev-dependency. `renderer/bridge/Cargo.lock` lists `c600-module` because `c600-runtime` depends on it.
- Not built:
  - a recording's route (LP4-9), which must bind through `Modules::bind` or `Modules::bind_pinned` and extend
    CT-EV1-04 when it arrives (the debugger's route binds through `Modules::bind_pinned` since LP4-5);
  - a built-in entry's binding on the runtime's process: an approval carries no class rows, so `BoundJob::new` refuses
    it at the admission stage and it runs in process only;
  - the conversion of a program job's JSON arguments to an entry's declared parameters, which waits for the module
    formats' declared parameters (LG5.2); `ModuleRun::of_job` takes the values converted;
  - persistence of the set: candidates and active versions live in memory for one session, and a reopen has nothing
    active until the host checks and activates again;
  - stopping a running binding when the host revokes a grant (KN7's, the run's host's), and the revision-driven runs of
    an overlay view, which the view model does not run yet;
  - the shell's write-check-run flow and its explicit activation control (LP4-3).
- Evidence kind: source. The process-host tests start the runtime's child on the test machine; nothing is timing,
  input or Windows interaction evidence.

## Status of LP4, LP4-4: overlay rows and the palette

- Overlay rows (KN5.3, EV4.1): `CommandLayer::register_overlay_with` registers a language-defined command with its
  label and the capabilities the solver granted its module (`Registration`, `Grant`: file, clock, randomness; consent
  at install, ADR-D01.32). Its kind is view, workspace or query, or file with the file grant. A commit or app row, a
  file row without the file grant and a row that takes a token are refused with `KN5:input.overlay_row_kind`; an ID a
  stored row, a service or another overlay holds, the stored puzzle rows and `command.find` among them, with
  `KN5:input.overlay_row_conflict`. `register_overlay` keeps its signature and grants nothing, so a file row needs the
  new call.
- A language-defined row whose stored static verdict failed is refused at invocation with that diagnostic, by key and
  from the palette alike, so a module that failed its check runs nothing.
- The palette's data (EV4.2, KN5.6): `Palette::read` lists every row of the table, H-08's 209 (with LP5's research rows)
  and the overlays, with its label, kind, key sequences and its answer in one KN5.6 availability table (a computed
  binding per row: the argument-independent gates). `Shell::read_palette` reads it once per revision and generation;
  `Palette::find` searches the kept read with no layer, case-insensitive substrings of the label, row ID or key
  sequences. Unavailable entries keep their places with their typed reasons. A palette invocation is `Shell::invoke`,
  the key route's own path.
- `command.find` is built: after its `query`'s form, and for a run only inside a command the solver invoked, it is
  `accepted` with no job and changes nothing. No entry can serve it.
- `Shell::bind` adds a key binding; since LP4-8 it runs EV4.7's checks of the whole map (below).
- CT-EV4-01 runs whole in `c600-command/tests/palette.rs` (source).
- Not built:
  - ranking by the solver's own recency, frequency and pins (ADR-D01.18), glossary terms in the search, aliases
    (`palette.alias`), pins (`palette.pin`), synonyms (ADR-D01.17), short help and the selection filter of
    `command.find`'s `object`;
  - revoking a grant or removing a registered row; the host keeps its registrations for the layer's life;
  - a run's own invocation of a workspace or file row. ADR-D01.2's first route lets an invoked entry point run view
    and workspace rows (overruling LG3.1 and LG3.5 in part); the layer still follows the owner's reading of ONE.52 of
    18 November 2026 (findings B3 and B4), so a workspace entry point the solver invokes writes its records in its own
    completion, and a run reaches view and query rows only. Whether ADR-D01.2 lifts that reading is the integrator's
    question;
  - the palette's generation event (KN5.8's protocol): the shell compares the layer's revision and generation when it
    reads.
- Evidence kind: source. Nothing here is Windows input, display or timing evidence.

## Status of LP4, LP4-15: durable rollback step IDs (R2-5)

- The owner's decision 4.2 of 23 November 2026: each step of `session.rollback` is journaled under the command ID
  `{gesture}#{i}/{k}` (1 <= i <= k, plain decimal), so "j of k" is durable and a reopen tells a gesture cut at j of k
  from a whole one by the step IDs alone. `rollback_step` parses the form to (gesture, i, k); the journal format does
  not change, since the step ID is the command ID the journal already keeps.
- A reopen admits a step as the live gesture writes it: step 1 under a gesture ID that no earlier action or gesture
  used, and each later step right after the one before it with the same k, in the same epoch. Step 0, i above k, a
  number not in plain decimal and the older form `{gesture}#{i}` name no step and stop the replay in read-only
  recovery (`KN5:consistency.unadmitted_journal_action`). A press under a gesture ID whose gesture has steps is refused
  with `KN5:input.command_id_reused` whatever its k, live and after a reopen.
- `CommandLayer::rollback_steps` builds the step IDs from the gesture's k; its receipts' commands carry k.
- R2-5's reading test in `c600-command/tests/attack_rollback.rs` is now a passing test of the fix
  (`a_cut_gesture_and_a_whole_one_journal_apart`), and `tests/rollback.rs` gains the crash case and the admission
  cases. The `session.rollback` row's note in `command-rows-commit.json`, `command-table.json` and `command-table.md`
  names the new form.
- Not built: a journal written before this change with a rollback gesture in the older form reopens in read-only
  recovery; no migration of such a journal is built, since the row was a draft of 22 November 2026.
- Evidence kind: source.

## Status of LP4, LP4-8: bindings, layouts, rebind and macros

- One checked key map (EV4.3, EV4.4, EV4.7): the key-map model moved from `c600-input` to `c600_command::keys` (chords
  in W3C `code` names, the US QWERTY and French AZERTY tables, `BindingSpec`, `KeyMap`, EV4.7's checks and the draft
  default map); `c600-input` re-exports it in its old modules, so the router's API is unchanged. The shell's key
  route (`Shell::press`) and the router use the same map, which closes C5's open item "one
  map is to remain": the shell's own five-chord draft map, `KeyBinding` and `ArgSource` are gone, and Escape no longer
  cancels a preview anywhere (DT-M1 C4: `Ctrl+Shift+Backspace` does; Escape is `transient.close`).
  `Shell::press` takes a sequence of physical chords or character intents (`Ctrl+'z'`) resolved under the map's active
  layout, in the global routing class for the shell's destination; a relative name only the router holds (a grip
  slot, a menu item, a list entry, a phase, a scrub position) is refused before any envelope with
  `KN5:input.argument_form`. The default map gains `Ctrl+Alt+'z'` and `Ctrl+Alt+'y'` for the layout history's undo and
  redo (ADR-D01.14), by character as undo and redo are (Q2 (b)).
- `Shell::bind` replaces LP4-4's draft: it checks the whole map with the new binding (`KeyMap::bind`), duplicates and
  the prefix-code property per routing class and every check under each declared layout, and refuses with the first
  finding's code (`EV4:input.binding_conflict`, `EV4:input.binding_prefix_conflict`,
  `EV4:input.binding_layout_mismatch`), leaving the map and the palette as they were. `Shell::apply_keymap` takes a map
  that passed the checks, such as one under another active layout. The palette shows key sequences as chord names
  (`Ctrl+KeyZ`).
- The applied grip-slot map (EV4.8, PS1.26; DT-M1 K2, Q3 (a), K6): `grip.rebind {anchor, map, expected_map}` is built.
  It writes the record `grip/map` (anchor, caps and transported frames in slot order, and the model) as one record
  write, undoable as a workspace inverse, after checking the forms (`KN5:input.argument_form`: a piece anchor, 1 to 600
  slots of exactly `{cap, frame}`, caps below 600) and that `expected_map` is the digest of the applied map
  (`CommandLayer::slot_map_digest`; `KN5:stale.revision_changed` otherwise). Before the first rebind the host's default
  map applies: the slot map the router's `Context` is made with. `Context::rebind` is gone; `Router::slots` reads the
  layer's applied map at each press, so only the rebind command changes what a slot key or a twist key's grip sends.
- The macro base (EV4.9; JI.18 to JI.20; DT-J1 E1, DT-M1 K6): `macro.save {word, name}` is built, with H-08's `confirm`
  gate (`KN5:input.confirmation_required` without `confirm: true`, which the journal keeps). It writes `macro/<name>`
  holding the alphabet (`full` or `menu`), KN1's identity block (the model's, or the jumbling block with the pinned
  menu and its order), the literal letters in chronological order, flat (signed generators, or JI.19's `60*grip +
  item` codes), the exact expanded length, the literal digest (the journal's for a Full word; JI.19's
  `ia16-expanded-le/1` for a menu word, which reproduces JI.23's digest), the start digest (labels with Jumble off,
  poses with Jumble on), the source as written (a recipe's words and stars are its named parts) and the key contracts
  (`word-key/1`; `trace-key/1` under `full-disjoint-support/1` or `jumble-locality/1`). A save neither selects,
  inserts nor executes; it is allowed in either Jumble mode, so one base serves both puzzles, and a save under a used
  name writes the next version. `CommandLayer::macro_base` lists it. `macros::check_keys` refuses comparing two
  macros whose alphabet, identity, key kinds or versions, or independence contract differ with
  `LB1:input.key_contract_mismatch` (registered in chapter 13).
- Only their own rows write these records: `workspace.set` and `worksheet.save` refuse names under `grip/` and
  `macro/` (as under `set/`), a program job's output that writes one fails with `KN2:input.named_object_invalid`, a
  run reaches neither row (`LG3:capability.not_granted`), no entry can serve them, and a reopen stops in read-only
  recovery (`KN5:consistency.unadmitted_journal_action`) at a write of one that is not its own row's from the shell,
  with its confirmation and a record that reads, and at a job's completion under either row, which runs no job (B7).
- No H-08 row was added: `grip.rebind` and `macro.save` were stored rows. The command table did not change.
- Tests:
  - CT-EV4-06 whole (synthetic): `c600-input/tests/keymap.rs` on the specs and `c600-command/tests/keymap.rs` through
    `Shell::bind` on the shell's one map, with DT-M1's US and French counterexample refused under AZERTY while US is
    active.
  - CT-EV4-07 whole (source): `c600-command/tests/rebind_and_macro_base.rs` (the rebind leg on the layer, the
    reserved-name and replay legs, the macro base for both puzzles across Jumble on and a reopen, the checks of a save,
    the key contracts) and the rebind leg of `c600-input/tests/routing.rs` (a slot key sends the rebound cap).
  - CT-EV4-05, headless part only: `c600-input/tests/ime_focus.rs` (synthetic IME composition, text undo under both
    layouts and focus loss suppress dispatch and clear held keys). Not run: CT-EV4-05 itself, a windows test with
    actual Windows IME, text undo and focus loss, on the owner's machine.
- Not built:
  - the key-map rows `keymap.edit`, `keymap.save`, `keymap.defaults`, `keymap.name` and `file.keymap.*`, and the
    persisted binding and key-map forms of chapter 05 (CT-PS1-13's round trip); the shell's map is built in code;
  - the other `macro.*` rows (`macro.base.open`, `macro.select`, `macro.add`, `macro.replace`, `macro.compare`,
    `macro.variant.create`, `macro.metadata.set`, `macro.filter`, `macro.library.check`, `macro.compose`,
    `macro.operand.set`, `macro.reference.check`) and `file.macro.*`; a macro's use, where a jumble letter is refused
    with Jumble off, waits for `macro.add` and `macro.replace`;
  - `macro.save` of a `{"record": name}` word, named spans with `same_as` and `inverse_of` relations (JI.21), chunked
    storage and streaming (JI.20, JI.22) and DT-J1 X3's expansion bound;
  - the word and trace keys themselves: they are the interpreter's primitives, and `c600-command` keeps no path to it
    (ONE.48), so the layer records their contracts and `macro.compare`'s job will compute them;
  - `grip.select`, `grip.anchor.set` and the transport of frames from an anchor (the map is stored as the rebind gives
    it); `FrameRef` has no settled form, so a frame is kept as given;
  - the renderer bridge's rebind: it keeps its one default slot and sends no `grip.rebind`.
- Callers moved to the map's chords: `c600-tools`' `c600-lp3-source` dismisses a refused commit's preview with
  `Ctrl+Shift+Backspace` and undoes and redoes with `Ctrl+KeyZ` and `Ctrl+KeyY`. The renderer bridge's host
  (`renderer/bridge/host`, LP4-3's `m6h_register_row`) gives its shell the router's map, and a row's key is bound by
  checking the router's specs with the new binding under both declared layouts (`KeyMap::apply`) and applying the
  result to the router and the shell together, so the palette and the router read one map. The spellings in the
  bridge's tests, the editor panel and `renderer/shell/README.md` are the map's.
- Drafts: the record names `grip/map` and `macro/<name>`, their payloads and provenance blocks, the stale code for a
  raced rebind, the 600-slot bound, the overwrite of a macro by name, and the Full word's literal digest profile, in
  the crate documentation of `c600_command::grip` and `c600_command::macros`.
- Evidence kind: source and synthetic. No Windows, IME, device or timing claim.

## Status of LP4, LP4-3: the shell's write-check-run flow

LP4-3 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (chapter 04 EV1.2 to EV1.13; chapter 05 KN5;
EV4.1, EV4.2), the last item of phase I, a draft. Its code is in `renderer/`, outside this workspace (CT-RPI-01); it
uses this workspace's crates through their public interfaces and changes none of them.

- The editor library `c600-shell-editor` (`renderer/bridge/editor`, a member of the bridge's workspace) is a C
  interface, version 1 (`m6e_*`). It holds one document with EV1's services from `c600-editor` (`m6e_set_text`,
  `m6e_edit` in the declared units, `m6e_diagnose`, `m6e_complete`). `m6e_check(entry)` checks the buffer as a
  single-file module with one entry, the solver's own program in the solving profile, through `c600_module::Modules`;
  it creates a candidate and activates nothing. `m6e_activate(module, expected, candidate, reference)` is the explicit
  switch (`activate_module`, EV1.11, EV1.13). `m6e_run(job)` binds a program job's entry to the active version once
  (`Modules::bind`) and runs it on a worker thread, in process or, after `m6e_load_runtime`, in the runtime's own
  process (`c600_tools::modules`); `m6e_run_poll` gives the run's record. Answers are JSON (`m6e_answer`); a refusal
  is `{"code", "detail"}` (`m6e_last_error`).
- The host library `c600-shell-host` goes to version 7. `m6h_register_row` registers a module's entry as a language
  overlay row (`tool.<module>.<entry>`, a query row, `register_overlay_with`) and binds its key in the router and the
  shell's key map, one map since LP4-8. `m6h_palette` reads the palette. `m6h_invoke` invokes a row from the palette
  (`Shell::invoke`). `m6h_take_job` and `m6h_finish_job` carry a program job's run (`ModuleRun::of_job`, as JSON) to
  the editor library and its end back to the layer as the job's result or error. `m6h_set_draft_word` sets the editor's draft,
  `m6h_set_focus` the router's text or puzzle focus, and `m6h_answer` gives the JSON answers. The host now builds the
  previews the layer queues (`run_jobs`) at each pump, frame and key, shows a pending preview on the view
  (`show_preview`, with no primitives yet), and reports each confirmed frame that showed it to the layer
  (`CommandLayer::present`, the base translated from the display's engine revision to the layer's state revision), so
  the commit key finds a presented preview. The two libraries share no Rust object; the panel and the flow test carry
  the JSON between them.
- Preview route: the solver-invoked route. A run's engine host serves no row (`NoHost`) and the run holds no token.
  The word the macro returns is shown and becomes the editor's draft; the solver previews it with
  `operation.preview`'s key (Ctrl+Enter) and commits it with `operation.execute`'s key (Ctrl+Shift+Enter, the key
  route's token) once a frame showed the preview; `session.undo`'s key undoes. The shell chooses, ranks and outputs
  no solving word of its own. The program-prepared route that ADR-D01.8 allows ("the solver's own programs prepare
  solving previews"; ADR-D01's overrule of EV4.16 and BND.84) is blocked in this workspace: `c600-check`'s
  solving-profile admission still refuses `PreparePreview` for an own, forked or imported tool (BND.56, LG3.16; the
  `Profile::Solving` branch of `c600-check/src/lib.rs`). Lifting it is a change to `c600-check`, which LP4-3 does not
  make.
- The greybox panel (`renderer/shell/project/Editor`, `M600_EDITOR=1`, `launch.py --editor`): a code field, the
  diagnostics list (code, line and column), code completion at the caret, and a palette list that starts with the
  panel's own Check and Activate entries, which have no key (the LP4 plan's integrator default), followed by the
  session's language rows. Activate registers the activated entry's row, the first one with the key Ctrl+Shift+KeyR.
  Each frame the panel takes the host's program jobs, runs them through the editor library and reports their ends.
  While the code field has the focus, the router has the text focus, so the field keeps typing and a commit row's key
  (DT-M1 C4). `build_product.py` builds `editor/c600_shell_editor.dll`.
- LX-4's editor library (`renderer/bridge/lx4`) is re-pointed onto `c600-editor`: its syntax, completion and
  diagnostics stages run `parse`, `complete` and `diagnose` (the W modules are the context's dependencies), and its
  dependents stage keeps `c600_lx4::front_end`. The `lx4_*` interface (version 2), its texts and its endpoints are
  unchanged.
- Tests: `renderer/bridge/editor/tests/flow.rs` drives both libraries through their C interfaces over the bridge's
  shim and the fake core. The macro is typed with an error and shows its diagnostic (`LG2:type.mismatch` on its line);
  completion offers `word`; the fixed macro checks and is not active, so a palette run fails with
  `EV1:input.module_not_checked`; a stale activation is refused and the explicit one succeeds; the row shows in the
  palette with its key; it runs from the palette in process and by its key in the runtime's process (the word
  `[1,-2,3]`); the draft is previewed by key, a commit before a frame showed the preview commits nothing, the commit
  and the undo go by key, and the close's control counts the two changes and reopens the journal at the start.
  `tests/test_renderer_shell.py` checks the panel's fixed points and both libraries' exports against the Rust source.
- Not built:
  - the program-prepared preview route (above) and ADR-D01.12's one-key preview-and-commit;
  - a module with imports or capabilities: the check builds a single-file manifest with no dependency and no
    capability, so a draft that imports or requires one is refused;
  - more than one entry per check, and arguments (a registered row takes none; their conversion waits for LG5.2);
  - persistence of the module set and of the registered rows, which live for one run of the shell; removing or
    re-keying a row;
  - the panel's look (ADR-D01's later rounds); hover, definition and rename in the panel; diagnostics drawn in the
    code field; the preview's primitives on the view;
  - the on-screen flow: the shell built offline with the panel (`build_shell.py`, no warnings), and it was never
    launched.
- Evidence kind: source. The flow test's frames are a fake core's and its keys synthetic; nothing here is Windows
  input, display or timing evidence.

## Status of LP4, LP4-5 and LP4-6: the debugger, replay and profile

LP4-5 and LP4-6 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (chapter 04 EV2.1 to EV2.12; chapter 02
LG4.12).

- Stepping (`c600-run`'s `step`, additive). `Loaded::run_gated` calls a `Gate` before each operation is admitted, at the
  run's own DET.20 position (the root's admitted steps) under the operation's DET.56 locator, with its source range,
  the enclosing call sites and the frame's bound slots by their source names. The gate adds no step, charge or tariff:
  a gate that lets everything through leaves rows, charges, stop and output unchanged. The run stays on the caller's
  thread, since the interpreter starts no thread (CT-DET-21); `c600-debug`'s `stepper` drives a gated run on a thread
  of the debugger's, one advance at a time: a dispatched host row is a suspended kernel call (`waiting(call)`), and a
  duplicate request returns its latest result without dispatching or charging again.
- `c600-debug` is the read-only debugger. `Debugger::debug_step(run, expected_stop, request_id)` answers EV2.12's
  `StepResult` (`stopped`, `waiting(call)`, `terminal(outcome)` with five distinct reasons, or
  `unavailable(step_mapping_missing)`), with the stop's source ranges and its call sites' or an explicit
  `unavailable(source_map_missing | source_bytes_missing)`, and stale requests as `EV2:stale.debug_stop`.
  `inspect_value` keeps exact values, approximate display values with their precision, partial results and verdicts
  apart (EV2.2). A stepped run's host is `ReadOnly`, which refuses every preview and sink with
  `EV2:capability.debug_write_forbidden` before the host behind it sees one. `words` resolves H7 references into
  literal and compressed words lazily, by occurrence path and offset (EV2.11). `Replayer::replay_run` preflights the
  record's key (module digests, edition, IR and contract, identity, input, cost units), re-evaluates the computation
  against the record's captured inputs, inspects its recorded effects without executing them (EV2.9 as the owner read
  it on 23 November 2026), and compares the trace, outcome, code, stop, coverage, charges, rows, result bytes and output
  (`EV2:evidence.replay_mismatch` with the field and position); a safety-stopped record is not run and stays incomplete.
  `profile` is EV2.7's cost profile, equal across cached, uncached and slow runs, with elapsed time as display data
  only. `journal` reads a host's query rows through the read-only `JournalRows` (EV2.8) and says whether a pinned input
  is still the head (EV2.10). `checker` steps KN6.4's per-claim trace, bound to its certificate and checker digests.
- `c600-tools`' `debug` is the host's side: `Rows` serves `JournalRows` over `CommandLayer`'s reading methods,
  `bind_debug` is the debugger's route through `Modules::bind_pinned`, and `launch` steps a binding behind its boundary
  with the read-only host outside it; `target` names an executor without a step mapping for the runtime's process.
- Tests, each naming its ID: CT-EV2-01 whole (`c600-tools/tests/debug_host.rs`, every debugger operation with a live
  engine behind it; its stepping part also in `c600-debug/tests/stepping.rs`), CT-EV2-02, CT-EV2-11 and CT-LG4-07 whole
  (`c600-debug/tests/stepping.rs`), CT-EV2-03, CT-EV2-04, CT-EV2-05 and CT-EV2-08 whole (`c600-debug/tests/replay.rs`),
  CT-EV2-06 whole (`c600-debug/tests/words.rs`), CT-EV2-09 whole (`c600-tools/tests/debug_host.rs`), CT-EV1-04's
  debugger route (`c600-tools/tests/ev1_host.rs`), CT-EV2-10 whole (`c600-debug/tests/replay.rs`, under DET.7 as amended
  and overlay OV-1), and parts of CT-EV2-07 (`c600-debug/tests/checker.rs`, `c600-debug/tests/dependencies.rs`).
  `c600-run/tests/step.rs` covers the gate and `c600-debug/tests/stepper.rs` the stepper.
- Drafts, which change until the editor protocol and the IR freeze (PHS.49): the stop and value reference texts
  (`<step>/<locator>`, `<call>/await`, `end`; `<stop>#<index>`, `<stop>#result`, `end#value`), `StepResult`'s fields,
  the typed `RunRecord` and `ReproKey` the replayer reads, the host-trace events a record holds, the cost profile's
  budget name (the run's), the journal row forms, the checker trace form, and the literal digests of menu letters and UI
  actions. Every host row a stepped run performs is a suspended call; a cancellation waits for a call in flight.
- Not built:
  - KN6.4's per-claim trace producer: no checker in the workspace writes one, so CT-EV2-07 runs in part, over a trace
    its test builds, and re-deriving a trace by running a checker as budgeted work is not built;
  - chapter 05's `RunRecord` and chapter 09's `ReproKey` forms, the session's capture of a run's host trace and a
    conversion from `c600-session`'s records (the debugger cannot depend on the session store); only tests capture;
  - opening an old event's snapshot state: travel opens journal entries, and the headless layer serves no snapshot of
    an old event; the `checkpoints` row is not built there and answers `KN5:input.row_not_built`; chapter 08's snapshot
    and branch lifecycles;
  - a step mapping for the runtime's own process and for RT2 and RT3: their sessions are
    `unavailable(step_mapping_missing)`;
  - EV2.5's provenance view of evidence and computation edges (only its word references are built), a `Derived`
    value's derivation and reachability claim in inspection (KN2.11 to KN2.14), and exact transport of a pinned result
    by a lemma (EV2.10);
  - the debugger's editor protocol and its C ABI (LP4-3), and replay of a jumbling record with its menu alphabet beyond
    the identity preflight.
- Evidence kind: source; CT-EV2-05 is synthetic. Nothing here is Windows input, display or timing evidence.

## Status of LP4, LP4-7: the declared-part round trip

LP4-7 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (chapter 04, EV3.5's `edit_declared_part` and
EV3.7), in `c600-editor`, a draft.

- `edit_declared_part(document, digest, declaration, field, value, context)` returns the source edits that set one
  field of a declared part to a typed `PartValue`, the SHA-256 of the edited bytes (the new version, `source_digest`)
  and the edited draft checked again under the same module context (its diagnostics and verdict, as `diagnose` gives
  them). The edit replaces exactly the field's literal with its canonical spelling, under the request's digest;
  `apply_edits` applies it. Nothing is persisted and no program runs (EV1.9). `declared_parts(document, digest)` lists
  the parts a source declares, each field with its path, declared type text, current value and range.
- Refusals, each an `Envelope` with service `EditDeclaredPart` and no span: a part the source does not declare is
  `EV3:input.round_trip_undeclared` with its reason (`Undeclared`: no declaration, no directive, a malformed
  directive, a field the directive does not list, or a part that is not a literal); a value of another type than the
  field's is `LG2:type.mismatch` (`Reason::ValueType` with the field's type text and the supplied kind, next action
  `SupplyValue`); a request for other bytes is `EV1:stale.editor_source`.
- DRAFT, which freezes with the syntax (PHS.49): the source syntax has no clause for an editable part, so a part is
  declared by a directive comment on the lines above its declaration, `# @editable parameter|layout|default
  [field.path ...]`; with no field listed, the one field is `value`, the whole result. A declared part is a `fn` with
  no generics, parameters, effects, charge or `let`, whose body returns a literal (exact or approximate number, `Q5`,
  text, boolean, literal word) or a record of literals. A computed value, a command, a view and a declaration without
  a directive are never round-tripped: nothing is inverted from a drawing (EV3.7). The integrator may replace the
  directive by a syntax clause at the syntax freeze; the service's data does not change.
- Tests: CT-EV3-05 runs whole in `c600-editor/tests/ct_ev3_05.rs` (source). It reads the three built-in tools'
  sources from their crates (`full_600_cell.6cs`, `representations.6cs`, `jumble_checker.6cs`): as shipped they
  declare no editable part, so every declaration and declared parameter is refused; each tool's fork (its shipped bytes
  with a declared parameter, layout and default appended) round-trips every declared field, reads it back and checks
  clean; drawn views, computed values, undeclared fields and malformed directives are refused. CT-EV1-07 now also
  covers the two round-trip refusals.
- Not built:
  - declared parts in the shipped built-in tools: their `.6cs` bytes are hashed into the build strings and LX-4's
    module set reads them, so adding parts to them is the integrator's change (the tests use forks);
  - the workspace command that persists a round-tripped layout or parameter with its undo (EV3.7, default 16), the
    host's fork operation, and activating the new version (LP4-2's `activate_module` does that once it is checked);
  - follow-pick navigation from a drawn instance to its declared part (EV3.5's `follow_pick`, chapter 10's
    `SourceMap`);
  - keeping the directive comments through `format`, which still drops comments (LP4-1).
- Evidence kind: source. Nothing here is Windows, input, display or timing evidence.

## Status of LP4, LP4-12: the 1.0 log and its compiler

LP4-12 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (ADR-D01.80 to .82 as revision 6's R-2 and R-3
amend them), in the new crate `c600-log`, a draft.

- What a log holds (`Log`): the model identity (KN0.1); the start state (solved, or its labels in PS1.7's
  `u32le-base64` carrier); the move history and the layout history (ADR-D01.14 to .16); the saved cursors and the
  full-state hash at save; the view, the layout and the workspace as recorded values; each open document with its
  identity (a path, or an untitled id that was never saved), the digest of its last saved bytes and its full contents,
  with its dirty mark; the embedded macros (FP.12's steps, Nat count and `DigestRef(recipe_expansion)`), Library
  entries and scripts (text and `DigestRef(source)`), each with its author class (`built_in`, `own`,
  `forked_built_in`, `imported`, or `unknown`); the preferences as canonical JSON text, recorded and never applied;
  where the log came from.
- Histories: each is an append-only list of actions. A node action (commit, partial undo, workspace action, layout
  step) adds a node whose parent is where the cursor stood; a cursor action (undo, redo, roll, select) moves the
  cursor, an undo only back on its line, a redo only forward, a roll either way, a select anywhere. An abandoned branch
  is the nodes no later cursor returns to. A partial undo names its commit and how many of its literal letters stay;
  its parent is the commit's parent, and the cursor must stand on the commit or on another partial undo of it. A
  commit carries its legal witness (0.4's word and star steps), its Nat counts and its pre and post full-state hashes.
- The record (`write`, `read`), DRAFT: an FP.11 record of format `c600-log-draft`, version 1, with null provenance,
  outside the data core like FP.37's playback state; payload `{log, seal}`, where the seal is the
  `DigestRef(content)` of the FP.5 bytes of the record with `payload` reduced to `{log}`. The bytes are FP.5's
  `c600-json-1`, or FP.7's `c600-gzip-stored-1` above 250,000 bytes, read under FP.9's limits and only in their one
  canonical spelling; FP.10's dispatch comes first, unknown fields are refused, and an unknown optional extension is
  kept. The extension is `.c600log` (`EXTENSION`).
- `compile(kernel, bytes)` (ADR-D01.81) runs the gates `format`, `model_match`, `moves` (every node replayed from the
  start with its legal witness through `c600-kernel`, a state kept only while a later node needs it), `final_hash`,
  `macros` and `scripts` (each embedded script given to `c600_editor::diagnose`, with the other embedded scripts as its
  digest-pinned dependencies; nothing runs). Codes: a bad record `PS1:input.persistence_format`,
  `PS1:input.noncanonical_value` or `PS1:resource_limit.persistence_size`; an unknown format or version, or an unknown
  required extension, `PS2:input.persistence_version`; a changed byte `PS1:input.digest_mismatch` (the seal, or a clean
  document's contents against its saved digest); another model `KN1:input.identity_mismatch`; a move without its legal
  witness the kernel's `KN2:input.recipe_shape` (or its other recipe codes); a replay that disagrees with a pre or post
  state, or a wrong final hash, `PS3:input.checkpoint_replay`. The result names the SHA-256 of the bytes it compiled
  (`binds`). Only when `format`, `model_match`, `moves` and `final_hash` all passed does it bind the saved state;
  otherwise the log is read-only with its problems and no state (R-2). A script's or a macro's problem leaves the
  state bound and the compile not passed.
- `Recorder` writes a log action by action with the kernel computing each state, for fixtures and short sessions.
- Tests (source): `tests/compile.rs` writes a session with an abandoned branch, partial undos, a branch switch, layout
  steps, open documents, a macro, a Library entry, a built-in script and preferences, and compiles it through every
  gate to the session's hash; a changed model identity, missing and wrong witnesses, a flipped byte (one named, and a
  bit flip every 37 bytes), a wrong final hash, bad structure, bad documents and other spellings each fail with their
  code and no bound state; a dirty document's unsaved contents and saved digest survive saving and reopening.
  `tests/dependencies.rs` follows the manifests and fails on a path from `c600-log` to `c600-run`, `c600-command` or
  `c600-session`, or on any dependency that is not a workspace path.
- Readings: `c600-log` cannot depend on `c600-session` as the plan's selection 1 lists, because `c600-session` depends
  on `c600-run`; it keeps its own plain history types, and converting the session store's journal and branches into a
  `Log` is LP4-14's. R-3 is read in its final form: every open document's full contents are kept, clean or dirty, and
  a clean document's contents must match its saved digest. Embedded scripts are checked without a session identity (a
  model-independent request).
- Not built:
  - LP4-14's document rows (`file.log.*`: New, Open, Save, Save as, Compile, Close, and the import commit with its file
    check and confirm gates, recovery checkpoint and undo journal), the tabs, the Problems tab, the compile lamp and
    the signature; saving the live session at exit, and restoring open documents at start with a changed-on-disk mark;
  - the conversion between `c600-session`'s history and `Log`;
  - logs of the jumbled puzzle: the identity block reads, `model_match` refuses it, and the replay is 600-cell-Full's;
  - interpreting the recorded view, layout, workspace and layout steps, and checking Library entries (they are recorded
    values); the install-time consent a script of another author class needs before it runs (the log records the
    class);
  - a chapter 05 table row and an FP.4 row for `c600-log-draft`, which are versioned changes of frozen text.
- Evidence kind: source. Nothing here is Windows, input, display or timing evidence.

## Status of LP4, LP4-13: C600 and MPUlt interchange

LP4-13 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md), in `c600-log`: 0.4's proof logs and MPUltimate
v1 logs of 600-cell-Full checked, imported and exported with 0.4's rules (`log_io.py`, `mpult_log.py`), which are not
changed.

- `c600::check`, `import` and `export`: `C600-STUDIO-SESSION-v1` (extension `.c600.json.gz`) with `decode_log`'s and
  `validate_record`'s limits, order and texts: at most 16 MiB stored and 24 MiB expanded, UTF-8 JSON with an optional
  BOM, a solved root, at most 10,000 transactions and 2,000,000 primitives, every recipe normalized with its counts,
  and every pre, post and final state replayed. Import makes one commit per transaction and records the preferences
  without applying them. Export writes the line from the start to the saved cursor as `Session.export` does (a partial
  undo becomes a transaction of its kept letters), as canonical JSON, raw or in a stored gzip member with no time.
  Codes: another model `KN1:input.identity_mismatch`, another format `PS2:input.persistence_version`, a cap
  `PS1:resource_limit.persistence_size`, a replay mismatch `PS3:input.checkpoint_replay`, other faults
  `PS1:input.persistence_format`.
- `mpult::NativeProfile` reads 0.4's `C600-NATIVE-PROFILE-v1` with `profile_data`'s checks, plus its format and its
  `profile_sha256`, which is checked on the file's bytes because 0.4 hashed `translations` with integer keys.
  `mpult::check`, `import` and `export`: MPUltimate v1 logs of 600-cell-Full (extension `.log`) with `import_native`'s
  checks: header, description with exact decimals, base36 colors, timer, CRC, macro groups and shuffle boundary, the
  verified bridge words, the replay of every move with the redo tail, and the colors at the pointer. The caps are
  0.4's: 16 MiB, 20,000 sequence entries and 10,000 native moves. Over a cap is `PS1:resource_limit.persistence_size`,
  another puzzle (header, description or dimensions) `KN1:input.identity_mismatch`, another MPUltimate version
  `PS2:input.persistence_version`, a CRC mismatch `PS1:input.digest_mismatch`, colors that disagree with the replay
  `PS3:input.checkpoint_replay`. Import keeps the redo tail as moves after the saved cursor (a roll back to the
  pointer). Export writes the line through the saved cursor and on along the last redo branch, with the pointer at
  the saved cursor: an unchanged import is written back with its recorded sequence entries, counters and timer, and
  any other transaction letter by letter through the profile's translations, as `export_native` does.
- Tests (fixture): `tests/fixtures/make_fixtures.py` made the fixtures with 0.4's own code in the pinned engine
  environment, and `expected.json` holds what 0.4 returned and the 0.4 sources' digests. `tests/interchange.rs`: the
  0.4 sources and fixtures are unchanged; the fixtures import to the states 0.4 reaches, transaction by transaction,
  and compile as logs to the session's hash; export then import is the identity, byte for byte for the raw C600
  fixture and every MPUlt fixture, and by the JSON inside the gzip one; a session's letter-by-letter MPUlt export
  equals 0.4's `export_native` bytes; the refusals above each carry their code; the description constant is MPUlt's
  block.
- Readings: the MPUlt profile in the fixtures is 0.4's synthetic one (`test_native_bridge.make_fixture`), not an export
  of the actual MPUlt process, so this is source and fixture evidence only. Unlike 0.4's export, MPUlt export keeps a
  redo tail and an imported transaction's recorded entries (macro markers and moves of no letter), so that an
  unchanged import round-trips.
- Not built:
  - checks against a profile from the actual MPUlt process on the owner's machine;
  - C600 export above 16 MiB stored: 0.4 compresses with zlib, this crate writes stored blocks only and refuses;
  - JSON nested deeper than 128 levels in a 0.4 log, which 0.4's reader accepts and the kernel's parser refuses;
  - the interchange command rows and file dialogs (LP4-14).
- Evidence kind: source and fixture. Nothing here is Windows, input, display or timing evidence.

## Status of LP4, LP4-10: reference docs and principle records

LP4-10 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (chapter 04 EV5.2 to EV5.5 and EV5.13; chapter 03
LB6), a draft. The crate `c600-docs` depends on the syntax, checker, IR, module and digest crates (the plan's five)
and on the evidence library, whose word function the explanation words come from (EV5.13). It runs no program and has
no path to the command layer; its tests run the fixtures, so the command layer, the kernel, the jumbling crates, the
interpreter, the tools, representations and library crates are its dev-dependencies.

- Reference docs (`reference`). A module's reference is generated from its source and its checked metadata: source
  and closure digests (the closure of a manifest with the module's supported identity, LG5.3), edition, IR version and
  contract, requested capabilities, supported identities, and per function its signature, declared and inferred
  effects, capabilities, grant rows, state reads (capability separation) and the range of its declaration in the
  source. The comment lines directly above a declaration are carried as written. The principle record is linked by its
  path and digest, never copied. A draft that does not parse, lower or check is `Status::Broken` with its findings and
  no entry, and its page says it is not the active tool's documentation (EV1.7).
- Decompositions (`decompose`). An entry's decomposition is generated from the checked IR before executor lowering:
  every function the entry reaches in any module of the program, with the checker's inferred effects, capabilities and
  grant rows; its direct calls; "may call" edges for function values and references (the checker's `reaches` beyond
  the direct calls); its kernel and host operations as opaque leaves (`kernel:read_snapshot(<row>)`,
  `kernel:propose(<target>)`, `kernel:publish_view`, `kernel:write_workspace(<record>)`, `kernel:receipt(<field>)`, ...);
  and its base parts, the IR's own operations (`base:<prim>`, `base:Repeat`, ...). Its digest names the IR version,
  the contract and every module's source digest. `check_submitted` refuses a submitted graph that omits a derived node,
  effect or edge or names other source or another IR, with `LB6:evidence.decomposition_mismatch`.
- Principle records (`record`). A record is the data file `docs/progress/1.0/principle-records/<module>.json`, form
  `c600-principle-record-draft` version 1, with LB6.2's parts: the specification of each exported entry (function,
  inputs, parameters, budget, output, puzzle, menu, level, domain, tie rule, the meaning of an empty and of a negative
  answer, and what the answer means outside the cited statement's scope), the justification (lemma, certificate, replay
  or "by definition", each with its statement and an evidence reference: a repository file, its SHA-256, basis, check,
  source label and whether it is imported), the instance (model identity, level, input, expected claims each naming
  the derivation step it repeats, and the fixture test), the binding (source and closure digests, model identity,
  decomposition digest, statements) and the parts a person writes (step map with the unmapped nodes and steps on both
  sides, reason with its glossary terms, failure examples, what an answer does not mean, givens, steps, prerequisite
  terms and records, and a leaf specification with its oracle test per kernel leaf). Every part is given or a typed
  none, `{"none": "because ..."}`. `admit` refuses an absent part, a none without its reason, an empty list, an
  evidence file at other bytes, a fixture or leaf test that names no function, a claim that is not its step's result,
  a kernel leaf with no specification, a step map whose unmapped lists are not the computed ones, and a prerequisite
  term that is unknown, not learner-facing or above the record's level (`LB6:evidence.principle_record_incomplete`); a
  record bound to other source or another closure is `LB6:evidence.decomposition_mismatch`. A record whose
  decomposition digest is not the current one is `Stale`: every hand-written part is stale, and the step map and leaf
  specifications wait for the person who refreshes them. `prerequisite_cycle` checks that records form a DAG.
- Records shipped: the three built-in tools of PHS.22 (`full_600_cell`, `jumble_checker`, `representations`) and
  the library's twelve modules (`words`, `quadratic`, `word_keys`, `linalg`, `trees`, `recipes`, `completion`,
  `protection`, `guarded_setups`, `macros`, `seeds`, `orbits`). Each instance runs on the real geometry: the
  600-cell-Full tool inside the command the solver invokes, through the tools' host bridge, with the layer's preview;
  the jumbling tool onto the jumbling engine's whole-word check under W-J fixture I_a's menu; state-reading library
  entries through `Boundary` and the host bridge onto a 600-cell-Full command layer at solved; pure entries on
  600-cell-Full values (generators, the golden ratio of the vertex coordinates, the seed atlas's collateral, the solved
  pose), and `linalg` on a plain example matrix over F5, never a correction library's rows (the program never shows a
  combination of corrections: owner, 22 October 2026, O6 (b)). Evidence files are model assets, research data, the
  technical report and `c600-library`'s tests, pinned by digest: a change to one makes its record refuse until the
  digest is refreshed. Leaf specifications name their oracle tests by path and function, not by digest.
- The mathematician's pass of 23 November 2026, fixed: `linalg`'s instance is a plain matrix (M1); `recipes` says that
  what remains of a star is the seed's whole effect, its cycle and any collateral, and calls generator 3 the seed role
  (M3, M13); `jumble_checker` admits the second letter because a twist at c moves only the pieces inside H_c⁺, rigidly,
  and creates no straddler of c (M6); `seeds` rests on Kahn's argument for its topological sort, with the library's
  test as a check on finite instances only (M27), and on the technical report's "Triangular stage invariant", which
  keeps finished orbits in place at macro boundaries only (M7); `orbits` reads -1 as the table's
  index for 600 singleton position orbits (M8); `protection`'s failure example is [1], which moves slot 1 (M12);
  `guarded_setups`' empty list means a buffer, since the tables cover every non-buffer position (M14, checked on all 35
  trees); `representations`' distance 0 is a definition (M15); `trees` says pose (M16). Counted with `core.py` on the
  model: generator 1 moves 3,094 pieces (4,600 slots), and generators 1 and 45 move disjoint slots.
- Explanations (`explain`). `explain` builds a tool's explanation from its record and its admission state: the
  specification, each argument's claim with its word from `c600_evidence::word` at a pinned edition over a graph of the
  record's facts (statement, scope, bases, checks, job outcome, source labels and check references kept apart), the
  instance's claim (one instance, a completed or failed producing job, or none when the fixture did not run), the
  decomposition's roots as collapsed links (`expand` gives a node's calls, kernel leaves and base parts; a long word is
  a digest and a length until `expand_word` reads letters from bytes of that digest; a named, commit-pinned statement
  is a lemma link; an imported result links its stored bytes and source label with its producer absent), the parts a
  person writes and the stale parts. A record's evidence enters through chapter 06's `record` sink as the host's
  write, so a `formal_proof` or `independent_checker` check is refused and shown as refused. A computed or certified
  argument's claim covers the recorded inputs only. Where no human review or formal proof is recorded, the claim says
  so. The author class is shown as data and no word reads it; an explanation takes no score and grants nothing. A
  fork's explanation keeps the cited statements as references and records none of the parent's evidence.
- Tests. CT-LB6-01, CT-EV5-01 and CT-EV5-06 run whole (`c600-docs/tests/ct_lb6_01.rs`, `ct_ev5_01.rs`,
  `ct_ev5_06.rs`). Each record's fixture is `fixture_<module>` in `ct_lb6_01.rs`; it reads the record's input and
  compares the module's answers with the record's expected claims. `tests/facts.rs` prints, on request
  (`--ignored`), the generated fields a record's refresh needs.
- Readings (drafts, to confirm at the next freeze): the record is data beside the module, bound to the closure digest
  of a manifest with no principle field, since the manifest's `principle` digest cannot be inside what the record
  binds (LB6.2 "the record is data in the module manifest" is read as "named by the manifest"); the tools' own
  manifests (`c600_tools::tool::Manifest::principle_record`) still say `false`, since `c600-tools` and
  `c600-jumble-tool` are not this item's files; the evidence library is a sixth dependency beside the plan's five,
  since EV5.13 asks for chapter 06's word function.
- Not built:
  - the jumbling library's sixteen modules in `c600-jumble-tool` (`jumble_admissibility` to `menu_words`), which have
    no record yet;
  - the manifests' `principle` digest and the tools' `principle_record` flag, which wait for `c600-tools`' owner;
  - commit-pinned lemma citations beyond one: `seeds` cites the technical report's proposition pinned to the commit
    that last changed the report, since basis theorem needs a commit-pinned record (EVD.64, EVD.65); every other
    statement is typed;
  - the mathematician's check of the pass's fixes, and their review of the parts the pass left unchanged, which each
    record's `review` field names;
  - a rendered reference or explanation page in the shell or the docs site: `render` gives Markdown text only;
  - RT4's host decomposition (RTE.80), since RT4 is not built (LB6.6).
- Evidence kind: source and fixture. The fixtures' instances are the real model's and the research fixtures'; a pass
  proves no universal lemma and records no human comprehension result (EV5.4).

## Status of LP4, LP4-11: the glossary check and the learning path

LP4-11's glossary and fixture part (chapter 04 EV5.6, EV5.10, EV5.14, EV5.15 and EV5.18), a draft under the
mathematician's review. It is data and a standard-library Python check outside this workspace:
`docs/progress/1.0/glossary/` (`terms.json`, `learning-path.json`, `diagrams/`, the generated `glossary.md` and
`build.py`), tested by `tests/test_glossary.py`. The principle records of LP4-10 name their prerequisite terms in it.

- The term check (`build.py --check`). A definition writes a reference as `[[Term]]` or `[[shown text|Term]]`; it must
  name one row by its term, an alias or a homonym's declared bare sense, and nothing is resolved by a similar name.
  The check refuses an unknown term, an ambiguous term (a homonym word without a bare sense), a definition that uses a
  term above its own level unless the row records that forward reference with its reason, a learner-facing definition
  that uses a row marked `"learner_facing": false`, a citation without a registered source prefix (a document such as
  "Theory" or "Chapter 03", a clause prefix such as `KN`, or a repository file or link that exists), a homonym sense
  without its own row, and a bare homonym word in a definition (`[[covers|~]]` marks an ordinary word). It also checks
  the diagram manifest (EV5.15) and that `glossary.md` is the output of the data.
- Integrated entries. `terms.json` (version 2) holds 298 rows: 31 solving rows (the 30 before, with twist split into
  its two puzzles' senses and frame group replacing orientation group), 230 new rows from chapter 04 section 6's 246
  entries (the other 16 merged into solving rows with their scope changes), 20 sense rows of EV5.18 that had none, and
  17 rows for EV5.6's step concepts that had none (empty answer, not
  run, specification, not meant, workspace undo, conjugate, commutator, mixed statement, approximate value, the
  human-solve boundary, stabiliser, coset, quotient, homomorphism, F2, F5, Q(√5)). Section 6's ambiguous citations are
  qualified ("partition default 10", "DT-M1 Q7 (a)", "LQ14 (access policy)"). Every row has its diagram source, its SVG
  rendered by the local Mermaid CLI 12.0.0 and both digests in the manifest; the changed rows (cap, grip, label, piece,
  generator, orbit, Net and Strict, complete effect, orientation, frame, occupant, target) got new diagrams.
- The manual's first use (owner, 23 November 2026). A row's `manual` words name the manual's word for it; `build.py`
  computes where the manual's chapters first use each, by section, and refuses a definition that uses a term the
  manual introduces in a later section unless the row records it in `first_use_forward`. Eight such uses are recorded
  for review (lattice, N, seam, height, alignment domain and cap cover use pose, cut cell or retained twist before the
  manual names them; the ν-corrections are announced before ν).
- The mathematician's pass of 23 November 2026 and its re-check, fixed: mixed statement says a failed budgeted search
  (M2), and an exhaustive search refutes it only over the whole set "some" ranges over, under EVD.42's guard (M26);
  census rank is the Theory's application rank, r(A) = |M(A)| − 1 (Theory Lemma 6; `rank` = len(`mask`) − 1 on all
  35 orbits of `assets/census.json`), and is learner-facing, which replaces EV5.18's "what it counts is not
  established" (M9, M29); cap cover says what covering is (manual 2.2, M10), as a hint about the order that can fail
  (manual 2.2 and 2.6, M25); the 600-cell-Full twist is a rotation of the cap's A4, a sequence of H_c and T_c
  (M11), and the jumbling twist cites the menu Λ_c of card 6 (M19); residual names the frame group (M17); tree branch exists at
  height 2 or more (M18); commutator's order is settled by the atlas, [H0, T5] = 1, 12, 1, −12, and it speaks of
  disjoint supports (M20); the human-solve boundary allows setup search and the 17 October repair list (M21); In_c,
  Out_c, A_c, N and alignment domain name the alignment domain, a coset of poses (M24); admitted domain explains D[k].
  Their diagrams were rendered again.
- The learning path (`learning-path.json`). EV5.6's seven steps, each with its concepts resolved to rows, the rows
  their definitions need (`closure`), revisits, waiting rows (the flow rule waits for LQ2), its command routes and one
  fixture task. The check refuses a cycle among the steps, a concept that is not learner-facing, a concept whose
  definition uses a term no step up to it teaches (unless the manual or the solving glossary teaches it, as `assumed`
  names, or the step records the use, as step 1 does for the principle record and step 3 for assistance provenance), a
  revisit of a concept that neither an earlier step nor the manual teaches, a fixture concept taught after the task, a
  fixture whose recorded instance is missing, changed or lacks its expected value, and a fixture whose answer is a word
  or a macro. The fixtures read family R's coverage in the 7302 search record and the floor instance in the manual
  (both of the jumbling puzzle under menu I_a), CT-EV3-05's fork with its two marked parameters (LP4-7) and the
  600-cell-Full tool's record, the command table's key-change row (read live by ID, since the draft table changes), the
  seed atlas, the `seeds` module source, and the guarded setups' certificate with its digest (a match names the cited
  bytes and checks no certificate).
- Tests. `tests/test_glossary.py` runs the check on the repository and plants each refusal. It runs CT-EV5-02 in part:
  the review clause (the mathematician's pass is recorded in OV-3, and its fixes wait for their check), flow goals
  against the disclosure data (LP4-11's `c600-command` part) and BS3's branch (LQ2) are not run. CI runs
  `python tests/test_glossary.py`, which also runs the check.
- Readings (drafts; OV-3 records them): EV5.6's phrases are resolved to rows; the path teaches four concepts earlier
  than EV5.6 lists them, because definitions at step 1 use them (theorem, certificate, digest and identity level);
  assistance provenance keeps its frozen step 4, and the workspace record of step 3 records a forward use of it, since
  its "where required" applies only to stored words (the mathematician's ruling); step 4 depends on step 1 only and
  revisits Word, which the manual's chapter 1 shows with jumbling words only; the path adds eleven rows EV5.6 does not
  list (view,
  statement, evidence, admitted domain and sticker at step 1, source range at 2, keyboard key at 3, menu letter and
  revision reference at 4, state derivation at 6, gauge at 7); the manual's first use is read from its chapters, since
  its README previews terms before they are taught; bare chamber reads as cut
  chamber and bare key as keyboard key, bare block as achievement block (EV5.18) and bare witness as this glossary's
  witness; a record's specification uses the declared domain, proposed at L6, at L2, recorded as a forward reference.
- Not built:
  - the mathematician's check of the pass's fixes; rows the pass did not change still say in `review` that they wait
    for review, since the pass itself is recorded in OV-3;
  - UI placements for the new rows (`ui` is absent, "not placed yet"), which belong to the design line;
  - soft disclosure and the flow goals' undisclosed-concept check, LP4-11's `c600-command` part;
  - the human tests of the path (EV5.7, EV5.12), which wait for the owner's form;
  - prose outside the glossary rewritten with EV5.18's qualified names.

## Status of LP4, LP4-9: semantic recording, playback and conversion

LP4-9 of the [LP4 plan](../docs/progress/1.0/language-lp4-plan.md) (chapter 04 EV4.10 to EV4.23; chapter 05 PS1.30
and PS1.44; chapter 13's EV4 codes), a draft. Undo follows X1 (owner decision of 23 November 2026,
`docs/wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md`, item 2.7): the move history (`session.undo`) and
the layout history (`layout.undo`); each test that reads an undo target names its history.

- Built in the command layer (`c600_command::recording` for the data and the tables, `src/layer/playback.rs` for the
  layer's side): H-08's `recording.start`, `recording.stop`, `recording.play` and `recording.convert`, and `job.stop`
  for a playback's job. No command-table row was added and the table did not change.
- Capture sits in `CommandLayer::invoke`: each invocation of the shell's channel whose row resolves is one
  `RecordedStep` (row, canonical arguments without a token, an operation preview ID or a confirmation, the word and
  its block, the revisions it read, the module digest of an overlay step, the action it took (LI.9), its command and
  request, and its terminal receipt or none). A retry the outcome lookup answers, a run's invocation and a
  `recording.*` row are not captured. `recording.stop` stores the recording in the host's recorder.
- Playback is a program-lane job the layer runs itself (`run_jobs`). EV4.22's eligibility table
  (`recording::eligibility`, enumerated whole in CT-EV4-12) decides which steps play unattended: kind `view` or
  `workspace` on lane `view` or `none`, and kind `query` on lane `none`, with no job and no `confirm` or `file_check`
  gate. Every other row is a user step, and so are the integrator's additions: the rows whose change the layer
  confirms itself (`protection.policy.set`, `protection.orbit.set`, rows with an action guard) and the `recording.*`
  rows. An unattended child is a new invocation with its own command ID (`{playback}/play/{epoch}/{step}/{n}`), no
  token, written to the state as `pending` before it is dispatched; while it is dispatched the layer dispatches no
  other row and no token (`playing_admits`), and a run inside it never replaces a preview (ADR-D01.9). A user step
  waits for the shell's step action (`Shell::step`, `Shell::step_preview`, `CommandLayer::begin_step`): the shell fills
  the current revision, hint, presented preview and the solver's confirmation and mints a new token; a word commit
  takes a fresh whole-word preview of the stored word; an undo or redo is checked against EV4.18's map. Unattended
  playback cannot commit: a commit-lane row is never eligible, `playing_admits` refuses anything else, and a child that
  commits is held (EV4.19).
- EV4.19's two tables (`recording::historical`, `recording::child`), EV4.22's play table (`recording::play_from`),
  EV4.18's map (`recording::mapped`, `PlaybackState::map`), EV4.14's identity, module and staleness checks, and
  EV4.20's reconciliation: a pending child is looked up (KN5.23) after the invocation that settles it, after a job's
  report, at each `recording.play`, at `set_recorder` and at a restart, and advances at most once; an unknown outcome
  stops and is never retried; an ordinary invocation never moves the cursor; the cursor's writes go to the recorder and
  are never an undo target. A recorder write that rolls back leaves the layer's state as the recorder's last durable
  write left it (read back from the recorder); a write whose fate is unknown is read again from the recorder before the
  next recording row, `job.stop` or step action. A state a job left `running` when it ended on a failed write, with no
  child pending, stops with `KN5:resource_limit.engine_lost` at the next `recording.play`, as at a restart; that write,
  like the reconciliation, comes before EV4.22's table decides. `job.stop` answers its write's fate: rolled back, it
  stops nothing and answers `failed`; unknown, it ends the job and answers `unknown_outcome`.
- EV4.23's conversion (`recording::convert`) writes nothing unless every step passes; it writes `draft/recording/<name>`
  (an operation draft whose Macro phase holds the literal word) or `source/recording/<name>` (the generated module text
  and its calls) as one record write, undoable on the move history. The record's block is the host reader's
  (`Provenance::assemble`): `c600_tools::workspace::Reader` gives a draft a `manual` block and a source a program block
  of the generated module (entry `replay`, profile `solving`); both list the contributing words' blocks as parts.
- The recorder: `c600_command::Recorder`, with `MemoryRecorder` for tests and the session store's
  `JournalStore::recorder` (`c600_session::recorder::JournalRecorder`): recordings and playback states are workspace
  records in the session's own journal (PS1.16, PS1.44, FP.37), one draft record kind,
  `{"record": "command_workspace", "id", "kind", "value"}`, in `c600-command-journal-draft-1`. `kind` is
  `c600-recording-draft` (the value a record object the frame cites) or `c600-playback-draft` (the value inline, so a
  playback adds no object per step); `id` is the recording's name; the last record of an id and kind wins. A record
  takes no action number, has no receipt or undo target and no transaction cites it; it shares the journal's lock,
  fates, torn-tail cut and read-only recovery. The host's version reader is `c600_command::ModuleVersions`, with
  `c600_tools::modules::ActiveVersions` over a module set; `c600_tools::modules::bind_job` binds a recording step's job
  pinned to the digest it recorded.
- Tests:
  - `c600-command/tests/recording.rs`: CT-EV4-08 to -13 and -15 to -20, CT-PS1-15 and -24, and CT-ERR-18, several of
    them whole by union (`docs/progress/1.0/lp4-ct-index-notes.md`); CT-PS1-17 and CT-BND-03 in part (not run: a
    rename, which no row builds; a word's file round trip, since no file export or import of a word is built; the
    macro route, since `macro.save` writes its own block); CT-EV4-14 and CT-PS1-25 synthetic (the recorder's
    injected fates and a reopen); the format packet's `playback` (12), `playback_from` (26), `playback_trace` (3) and
    `playback_arguments` (4) vectors.
  - `c600-durable/tests/ct_ev4_14_playback_crash.rs`: CT-EV4-14 and CT-PS1-25's three crash points on the journal,
    which holds the recorder's records, over a memory disk; `c600-durable/tests/ct_bnd_03_recording_blocks.rs`:
    CT-BND-03 and CT-EV4-10 on C5's reader; `c600-durable/tests/ps1_44_carrier.rs`: the carrier, what reset, import
    and the recovery checkpoint do to it, and a source conversion whose recording's name holds line ends parses,
    lowers and checks.
  - `c600-session/tests/recorder.rs`: the journal recorder's records outside the action order, torn tail, failed
    writes and records that do not read.
  - `c600-command/tests/recording.rs` also holds the recorder's failed and unknown writes (a rolled-back state write
    and the next play's recovery, `job.stop`'s fates, an unknown `recording.stop` read again) and the generated
    module's one-line comments.
  - `c600-tools/tests/ev1_host.rs`: CT-EV1-04's recording route, so with `c600-module/tests/ct_ev1_04.rs` CT-EV1-04 is
    whole by union for every route (palette, key, view source, debugger, recording).
  - Updated in `c600-command/tests/`: `attack_playback.rs` (the four rows are the layer's now; a served
    `recording.play` is refused and a run's is `not_granted`), `settle.rs` (a playback's step stands in for the run
    inside a `recording.play` job), `served_rows.rs` (its served program-lane row is `recording.play`'s H-08 entry under
    another ID, since H-08 has no other row on that lane), `argument_forms.rs`, `filter_apply.rs`, `job_output.rs`,
    `one_key.rs` and `attack_reopen.rs` (which served `recording.play` as a stand-in row), and `kn5.rs` (CT-KN5-10's
    allowlist names `set_recorder`, `set_module_versions` and `begin_step`).
- Drafts and departures (recorded, frozen text unchanged):
  - PS1.44 and FP.37 put a playback state in PS1.16's workspace carrier outside the undo order; PS1.12 names the
    recording's writer and reader. Both live in the session journal's own `command_workspace` records, outside its
    action order. FP.12's carrier fields `id`, `kind` and `value` are there; `dependencies`, `action` and `revision`
    are not written, since the record takes no action, and the frame's place in the journal is its position. No
    frozen format changed: the record kind is a draft addition to `c600-command-journal-draft-1`, which a reader
    before it stops at, as at any record it does not read. The values are draft JSON (`c600-recording-draft` and
    `c600-playback-draft`, version 1), not FP.37's bytes; a recording is a record object, a state is inline.
  - The recordings and states belong to the journal they were written in. The command layer's path builds no reset,
    import or checkpoint row (`session.reset`, `session.checkpoint.save`, `session.checkpoint.restore`,
    `file.log.import`, `file.macro.import` are `KN5:input.row_not_built` and write nothing), a new session needs a disk
    with no journal, and a reopen reads them from the same journal; a reset or import that keeps the session must
    decide, when it is built, what happens to a playback state whose map names actions before it.
  - A state still `running` with no live job after a restart stops with `KN5:resource_limit.engine_lost`; the play
    job's receipt is `completed(workspace)` when the playback waits or finishes, and forwards a child's refusal or
    failure with its code and detail unchanged (ERR.7).
  - A negative integer `from` is `EV4:input.recording_state`, as the format packet's `play-from-negative` says, not
    `KN5:input.argument_form`.
  - `job.stop` stops a playback's job only; another job is `KN5:input.row_not_built`.
  - The step's `module` is the closure digest of the module's active version, which `bind_pinned` accepts.
  - A source conversion names each non-word call in a `#` comment and in its `calls`: the draft edition has no
    primitive that invokes a stored view or workspace row. Every control character and U+2028 and U+2029 in a comment
    is written as its `\u{...}` escape, so the module parses whatever the recording's name holds.
- Not built:
  - FP.37's wire form of a recording and a playback state, and the format packet's `playback_storage` vectors (18);
  - a host that installs `JournalStore::recorder` and `ActiveVersions` (the renderer bridge is outside this work), the
    recording rows' keys and UI, and the step action's UI;
  - `session.checkpoint.save`: a playback waits at it, and its step action is refused as not built;
  - CT-BND-03's macro route; CT-EV4-14 on a file-system disk on the owner's machine (memory disks only here);
  - recording a run's own invocations (a run is part of the job the solver invoked, which is the step).
- Evidence kind: source and synthetic. No Windows, input, display, long-session or timing claim.

## Status of LP5: research expressiveness

The plan is [`language-lp5-plan.md`](../docs/progress/1.0/language-lp5-plan.md); the owner started LP5 on 23 November
2026 with every recommended option. The three LP5 crates were added as empty members first, so the pieces share no
workspace file.

- R0, the research archive import (PS3.10 to PS3.17): `c600-archive` imports the stage 14 to 19 archive from files
  the test driver grants it out of the Git-blob manifest (`tests/test_lp5_archive.py`, in the headless CI). It uses
  versioned editions of the import table and of the discriminator for records with no `format` key (PS3.10, PS3.12,
  FP.22; ten rules, each pinned to the manifest's and the record's stored digests), checks manifests and members
  (PS3.14, PS3.15), keeps recognition, integrity, interpretation and recheck as separate results, and exports FP.23
  check bundles that carry the caller's KN0.1 identity, refuse any envelope or dependency whose integrity failed, and
  list missing dependencies; those with no recorded digest go in the optional FP.10 extension
  `missing_without_digest`. Gzip is decided by the member's `.gz` name, so damaged signature bytes are a gzip failure.
  It depends on `c600-digest`, `c600-evidence` and the kernel's container readers only, and calls no engine, process,
  Git or network. All 497 records import with no integrity failure. The dependency report
  ([`lp5-r0-dependency-report.md`](../docs/progress/1.0/lp5-r0-dependency-report.md)) finds every stage 18 start table
  stored or rebuildable by exact replay of stored words, so no LP5 check waits.
- R1, typed claim bindings and preflight (FR-B08, LB5.14 to LB5.17; owner, LP5-4): `c600-archive::bindings` reads
  edition 1 of the typed claim bindings (`data/claim-bindings-1.json`, pinned by SHA-256 and written by
  `data/claim_bindings.py` from the frozen claim matrix's anchors; `--check` confirms the file matches its build). It
  has one row per claim group (510), each with a typed statement, scope, inputs by digest and the KN6 sub-checkers
  that read the group, and answers LB5.14's `check_availability` as preflight data, never as a check and never as an
  evidence leaf. With every sub-checker registered, 286 groups are `ready` and 224 `unavailable` with the matrix's
  blockers; q11 answers `unavailable([dependency_missing, no_independent_checker], ...)` with its captured string
  unchanged (LB5.17). Groups a sub-checker answers `not_applicable` are bound scope-only and keep
  `no_independent_checker`. `c600-library`'s `research_preflight.6cs` states the same rule as language code over
  host-supplied data, and `c600-research/tests/preflight_bindings.rs` checks the bindings against the KN6 registry.
- R2, KN6's independent checker core: `c600-checkers`. `check(engines, certificate, kind)` returns a `CheckReport` with
  a pass, fail or not-applicable verdict, the checker's ID and source digest, the input digest, the stated inputs with
  the identities read from them and a per-claim trace; a kind with no checker, or one whose engine was not supplied, is
  a refusal with `KN6:evidence.no_checker`. Words replay letter by letter through `c600-jumble` and `c600-kernel` with
  no cache of the checker's own; the crate depends on nothing else but `c600-digest` and calls no chooser inside the
  engines. Four kinds are built: admissibility and straddlers, `macro-certificate/1`, compressed-word expansion
  (streamed, length-checked before any letter) and ν and label checkpoints; the three other certificate forms are the
  crate's drafts (`kn6-admissibility/1`, `kn6-expansion/1`, `kn6-checkpoints/1`). The mutation gate (`gate`) requires
  every fixture to fail under a tampered claim, a wrong start table and a wrong input digest, and under structural
  changes of its claims: a key deleted, renamed or joined by an unknown key, a list emptied or shortened by one element
  (a change that leaves the record unchanged is skipped; `Fixture::optional` names the parts a form may leave out, and
  no research-archive fixture names any). The research-archive kind is a family of sub-checkers registered per stage in
  `src/archive/s14` to `s19`, each with its own ID, source digest and gate (LP5 pieces R3 to R6). All 17 stage 15 macro
  certificates (R01 to R17) pass in place, read-only. `tests/test_language_scans.py` runs CT-KN6-02's import scan on the
  crate. An expansion's running time is bounded by the check job's budget (R8), not here.
- R3, stage 14's research-archive sub-checkers (CT-LB5-01): `c600-checkers/src/archive/s14` registers `s14.t3p`,
  `s14.literal`, `s14.locality`, `s14.levels` and `s14.blocks`, each with its own identity and mutation gate. From the
  stored stage 13 tables, the I_a menu and the model they recompute W3, W5 and the scan certificates, the T3P checks (P0
  to P4, the T3P+ edges, Lemma A's replay), the six chains' rounds, words and ends, the replay records' chronological
  rows, the locality summary, the fourteen level tables (greatest fixed point, λ and its witness) and the co-held,
  per-grip, jam and moves-522 records at the 7304 stop. All 44 records pass in place, read-only; changed helper sets,
  precedence, input digests or start tables fail. The 22 finite-search records (scans, pairs, plus, beam and peel)
  answer `not_applicable` with their scope, and only when they have exactly their signature's fields; any other record
  whose shape or format departs from its stage 14 form fails, naming the field. The fourteen-table level rebuild is an
  ignored test (about 100 s); the 7304 stop runs as its fast representative.
- R4, stages 15 to 17's research-archive sub-checkers (CT-LB5-02 to CT-LB5-04, CT-KN6-03's stage 15 to 17 parts):
  `c600-checkers/src/archive/s15` to `s17` register `s15.macro`, `s16.macro` and `s17.macro` (`macro-certificate/1`
  and `macro-check/1` through the macro kind, bound to the jam catalogue: stage 15 starts at the chain end and whole
  descents end at the jam, stages 16 and 17 start at the jam end, and a check record is matched leaf by leaf against a
  fresh check of its certificate), `s15.pstar` (the clean-pair census recomputed from the exact state), `s16.release`
  (the literal traces SJ-E1, T2-cx, I-7304 and I-7306 letter by letter, with S, F and max D at the named grips; SJ-E1
  stays a release), `s16.mobility` (unit mobility at the jams from the full label table), `s16.reductions` (reduced
  transport against the recorded unreduced case at 7307, never a lemma) and `s17.rr` (Lemma RR's five tails against
  brute force at `test_rr.py`'s recorded inputs H2 and H2b only, in an exact label calculus checked against the
  engine). P*, P**, P***, RR, routing, T1, alphabet and transport stop and run records answer `not_applicable` with
  their bounds; a malformed self-contained record fails at the departing field. `tests/kn6_r4.rs` holds the fast
  tests; thirteen slower gates are ignored tests.
- R5, stage 18's research-archive sub-checkers (CT-LB5-05, CT-LB5-09, CT-LB1-04's expansion half):
  `c600-checkers/src/archive/s18` registers `s18.expansion` (gate 1: both `steps.json` forms, each step's exact length
  admitted at most 10^7 before any word is read, every word file streamed and hashed), `s18.composed` (gate 2: the
  composed strict′ contract checked letter by letter), `s18.tail_replay` (gate 3: chunked literal replay for T6 steps 1
  to 3; T6 step 4 and the rule stages keep `historical_check_incomplete` and never pass it), `s18.lattice_run` (the
  `runner-run/3` record, report, tables, step trees and words) and `s18.lattice_replay` (the lattice phase's 13,231,266
  twists replayed from T6's stored end table, a fresh check with no archived `check.jsonl` to match). Start tables of
  later steps are rebuilt by exact replay and marked derived (owner, LP5-3). Stop units and whole-cap outputs answer
  `not_applicable`; any other departure from a record's form fails, naming the field. The R22B-10 clause covers the
  stored expanded X and V′ words only; no `r22b10/1` or `/2` certificate is stored. The in-place gate 2 and 3 replays
  (about 2 to 3 ms per letter in the test profile: about 1 h for T6's gate 2, 30 min each for the rule stages' gate 2
  and T6's gate 3, 11 h for the lattice phase) are ignored tests; fast synthetic records exercise the same code.
- R6, stage 19's research-archive sub-checkers (CX.15 to CX.17): `c600-checkers/src/archive/s19` registers
  `s19.pose-table` (a stored pose table's stored and decoded digests, the table checked in full, ending with the
  engine's state digest), `s19.gamma-matching` (Γ(Y)'s bijection and gauges brute-forced over the changed rows;
  CT-LB1-05's matching half), `s19.gamma-witness` (the nine-letter forward and inverse witness replayed on both states
  with the all-piece join), `s19.frontier` (the compact and cleanup exits, which fail the sticker goal) and `s19.ranks`
  (the fixed weights derived anew from the model's type counts, since `type-height-weight-inputs.json` is absent, and
  R3, S and R_L checked on the stored blocks with R_L's length premise). q11, the low tails, D[k] admission, the
  remaining obligations and reachability answer `not_applicable` with their scope; a malformed record fails and names
  the field. Seven type-lex records stay uncovered because the archive defines no irregular-piece vector I_t for them,
  and the cap-noninteraction claims wait for the delta_Q5 definition. `tests/kn6_s19.rs` runs every gate on the
  archive in place, read-only; three slow gates are ignored tests.
- R7, the research library's entries (LB1, LB5): `c600-library` holds them as 600 Cell Script in eight modules
  (`research_slp`, `research_bigint`, `research_fixpoints`, `research_flows`, `research_search`, `research_ranks`,
  `research_families`, `research_models`), with eleven catalogue rows in `c600_library::research`: LB1's straight-line
  programs, big integers and symbolic bounds, finite fixed points, and exact matching and integral flows; LB5's
  restricted word search, search reductions and gates, admitted-domain membership, ranks, bound calculation and
  compressed witnesses, regional-family recognition, and conditional models. No entry reads state; the search and the
  membership check run over synthetic permutation actions only, carry no witness word, and their four rows are marked
  research tier candidates. A search record keeps its declaration, outcome, stop reason and coverage; only a completed,
  unbeamed enumeration is a negative, `complete_lengths` counts only the lengths before the first beam discard, and a
  reduced negative leaves its family only through a matching lemma, otherwise `LB5:evidence.reduction_scope_mismatch`.
  Malformed input (no letter, or an image, start point, goal or target outside the action's points) is an error, never
  a negative or a rejection. Membership answers `OUTSIDE_CERTIFIED_DOMAIN` with k and its scope only
  after a complete enumeration, always with `reachability: not_asserted`. B11 (6,775 digits) computes without
  expansion. A row's effects stay within chapter 03's Effect column; its `budgets` is the most an entry may use.
  Drafts: an expansion over its bound answers as a value (`admitted: false`), since LB1 registers no code
  for it; D[k] is "within k letters of a target point"; a search budget counts examined candidates. Tests: CT-LB1-04,
  CT-LB1-05, CT-LB5-07, CT-LB5-08 and CT-ERR-11, their library parts. Not built here: the host's refusal of research
  tier rows in a solving session (BND.64, CT-BND-16), the entries' puzzle forms over KN2.4's reads and the jumbling
  form of D[k] (the research host's); strategies, selectors, certificate builders and seeded search.
- R8, reproducible research work (LB5.7, PS1.31, DET.1 to DET.7, DET.30, DET.48; KN7.3 to KN7.6): `c600-research`'s
  KN6 check job runs each check through `c600_checkers::check_until` as a child scope of the job. The input is pinned
  by digest: a session revision opened read-only (the job's engine-owned branch), or a start an archived certificate
  states. The job's kernel-cost budget reaches the checker as a cooperative stop, so a long expansion ends as the
  job's cut-off, never as a verdict, and nested checks mint no allowance. Its run record keys the adapter's and
  checkers' sources and identities, the cost and metering rules and the kernel build. `reproduce` re-seals and re-runs
  a stored record from its captured bytes and matches the exact payload byte for byte, with digests pinned for CI on
  Linux. Only a run's own reports reach `admit_check_report`; archived timed stops and data-only members stay recorded
  facts that supply no run. The checker core gained additive stop scopes and owned engines (`engines::Loaded`);
  outside every scope nothing changes. Tests: CT-DET-24 and the LP5 halves of CT-DET-01, CT-DET-02, CT-DET-11,
  CT-LG3-05, CT-LG5-02 and CT-EVD-30; CT-LG3-05's mode refusal is not run, since the store holds JumbleOff only.
- R9, the proof-obligation translator (LB5.18, PS1.45, PS1.46): `c600-evidence`'s `obligation` module, as pure code.
  It translates a typed Statement over `c600-obligation-vocabulary-draft-1` into PS1.45's `c600-proof-obligation-draft`
  Lean 4 text with `by sorry`, and matches the format packet's `obligations` vectors byte for byte. A named statement,
  an unknown symbol, a wrong arity or a form outside the vocabulary refuses with `LB5:input.obligation_untranslatable`
  and returns no text; a body whose digest differs refuses with the vector's own code. Symbol table draft 1
  (`c600-evidence/obligation/symbols-draft-1.json`) is compiled in and pinned by its digest; prelude draft 1
  (`c600-evidence/obligation/prelude-draft-1.lean`) is pinned by its digest, never compiled in and never run by Lean
  (LP5-2). The crate keeps its one dependency, so the module has its own strict canonical JSON reader. CT-LB5-11,
  CT-PS1-26, CT-RTE-23 and CT-ERR-19 run their translator parts (`c600-evidence/tests/obligation.rs`); the `file` row
  `research.obligation.export`, its grant and the proof that a refusal writes no file come with `c600-research`.
  `tests/sinks.rs` now scans module folders, so the translator is inside CT-EVD-20's and CT-EVD-21's scans.
- R10, the research output panel's rows (the plan's section 6): `c600-research::panel` adds eleven rows, six
  queries (`research.archive.inventory`, `research.claims`, `research.check.report`, `evidence.word`,
  `research.runs`, `research.obligations`), three file rows (`research.archive.import`, `research.bundle.export`,
  `research.obligation.export`) and two workspace rows with undo `none` (`research.check.run`,
  `research.run.reproduce`). `panel::Host` serves them through `CommandLayer::serve_row` in research-profile sessions
  only (a solving session is refused) and answers each from its own data: its file grant, its imports, the claim
  bindings and its KN6 check runs, never by running language code. Every answer is `{row, revision, research,
  answer}`, and `Host::read` gives charter D2's state, so an answer read after the revision moved shows as stale. File
  rows touch only granted files and a refusal writes nothing; check runs and reproductions go through R8's job
  adapter and are written as `research.run/<sha>` and `research.reproduction/<sha>` workspace records; the obligation
  export uses R9's translator with LB5.18's grant. LP5's block of rows is
  `docs/progress/1.0/command-rows-research.json`. `tests/panel.rs` is a headless driver of the panel's whole flow on
  fresh data; no CT is indexed for R10.
