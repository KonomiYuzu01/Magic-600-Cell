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
| `c600-module` | Module closures, lockfiles and their approval binding: the draft manifest and lock forms, resolution by content digest, closure verification and the closure digest, an approval bound to that digest, and load and run through the run-time boundary (LP3, C4) |
| `c600-runtime` | The runtime in a process of its own (route RT1): the child binary `c600-runtime`, the parent library that starts it and answers its host calls, the wire protocol draft `c600-rt1-wire-1` and the in-process reference path (LP3, C3) |
| `c600-durable` | LP3 C7's durable conformance cases on M6's session store, test-only: CT-RTE-08's persisted word, CT-EV3-07's durable action order and view part, and CT-LB2-03's work-sheet part (LP3, C7) |
| `c600-tools` | The built-in 600-cell-Full tool as `.6cs` source, its admission under the built-in column, the host bridge from the runtime onto the command layer, in process or on the runtime's own process, the recipe expansion, the source route `c600-lp3-source` and the tool's views (LP3, C5) |
| `c600-repr` | The representations tool, headless half: layouts and atlases with their source maps, the layout filter, linked selection, chart surveys and user-chart admission over the display contract, the tool's `.6cs` source and `view_aid` manifest, its class review, and what its six rows need from the command layer (LP3, C6) |
| `c600-jumble-tool` | The built-in jumbling tool, headless half: its `.6cs` source with two entries that preview a word the solver names, its `checking` manifest and built-in admission, the reading of a `Word` as literal jumbling letters, and the kernel halves of LB1's and LB3's jumbling tests (LP3, C8) |
| `c600-input` | The headless input router: synthetic key events to command-layer invocations in chapter 04's routing order, bindings resolved to absolute arguments at the press, the held-key allowlist and the key-map checks (LP3, C5) |

The checker depends on the IR crate only, and the syntax crate on the IR and digest crates; the syntax crate's tests
also run the checker. The interpreter depends on the IR and digest crates and the big-integer crates, not on the
checker; its tests run checked source programs, so the checker and the syntax crate are its dev-dependencies. The
kernel depends on the digest crate only. The session crate depends on the kernel, the interpreter, the IR and digest
crates; its job tests run checked source programs too. The evidence library depends on the digest crate only.
The command layer depends on the kernel and digest crates only, so it has no path to the interpreter (ONE.48). The
view model depends on the digest crate only, and the display contract on the digest, kernel and jumbling crates.
The module crate depends on the syntax, IR, checker, interpreter and digest crates, and has no path to the command
layer.
The runtime depends on the syntax, checker, interpreter, IR and digest crates and the big-integer crates, and has no
path to the command layer.
The tools crate depends on the syntax, checker, interpreter, IR, digest and kernel crates, on the command layer, and on
the runtime, display and view crates: its host bridge connects the interpreter, in process or in the runtime's own
process (C3), to the command layer as the runtime's host, and its views build on the display contract and the view
model.
The input router depends on the command layer and the kernel crate only, so a key reaches a kernel row with no path to
the interpreter (ONE.48).
The jumbling crate depends on the kernel and digest crates and the big-integer crates.
The representations crate depends on the checker, IR and digest crates, the display contract, the tools crate, whose
built-in column admits its entries, and the command layer, whose stored table its class review reads; it draws nothing
and owns no device.
The jumbling tool crate depends on the digest crate, the jumbling crate and the tools crate, whose built-in column
admits its entries; its tests also run the checker, syntax, IR, interpreter and kernel crates.
The durable crate holds tests only; its dev-dependencies are the session store, the module crate, the checker, the
syntax crate, the interpreter, the kernel and the digest crate.

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
  `LG3:capability.not_granted` an entry that is not exported, an ungranted capability, an entry that both reads state
  and prepares previews, a row the class does not list or the checker cannot name (an unresolved call, an open effect
  variable), and a proposal or a word-carrying workspace write whose word the solver did not name to the entry, unless
  an enabled declassifier covers the write; a write at a type variable, or of a record or callable type that holds a
  word or a type variable, counts as one that may carry a word. The rule is conservative: it also refuses a named word
  passed to a helper that proposes or writes it, a write of a record or closure built from named words, and a proposal
  to a `Target` parameter (a row the checker cannot name). Not built: the column's two exceptions (CT-BND-17,
  CT-BND-18), the derived-state row (the IR draft has no `derive`), rule 6's check of the facts a view shows and the
  class review (CT-BND-15).
- An entry that is no command and changes the workspace or prepares a preview is `LG3:effect.undeclared` at `entry`
  (LG3.5, LG3.6), on the built-in column and, for a workspace change, in `admit`'s solving profile. This confirms and
  closes LP3's finding of an exported plain `fn` that proposes: `check` admits it, since a command may call such a
  helper. In tool authoring and research `propose` goes to a capture (BND.60), and the frozen text does not say whether
  such a run needs a command entry, so `admit` still admits one there.
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
  the acknowledgement, the saved-word half and BND.61's refusal at run time (below); the undo half of CT-LG3-03 stays
  open.
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
- Canonical layout (LG1.4, CT-LG1-03). `format` depends on the parsed program only: four-space indents, a blank line
  before each declaration, imports directly after the module line, one `let` or `return` per line, `match` arms and
  closure bodies on lines of their own, and type arguments and generic parameters without spaces (`<T,E>`, as LX-8's
  type strings). Comments are not retained, as in candidate A, so the layout drops them; retention comes with the
  authoring services (PHS.21). `c600-syntax/tests/fixtures/canonical.6cs` pins one expected output by its SHA-256.
- Source map (LG4.12). `lower` returns, for each module, the source digest, each function's declaration range and
  the source range of every IR path in each body, so a checker diagnostic's function and path map to source text.
- What parses but does not lower yet. Lowering refuses each of these with a message naming what it waits for: a
  declared `charge` (LG4.8's tariffs), command metadata (the command table, LP3), binder arms and LX-8's domain types
  such as `Word<Full,Retained>` (the IR's domain families, M4 and M5), and menu words (the jumbled puzzle's alphabet,
  M5). Type arguments on a program's own function are refused: the IR draft has no explicit instantiation.
- Not run or not applicable, which is no pass (PHS.63): CT-LG1-01's second half, mixed dependency editions, while one
  edition exists; CT-RTE-18 (LQ13 was answered without it).
- Evidence kind: source and fixture. A pass is not Windows/DirectX, input, long-session or performance evidence.

## Status of M2: the interpreter and the meter

- The reference meter of chapter 09 (DET.55 to DET.60) replays the 40 frozen contract-vector traces with every field
  compared exactly, and the meter boundary vectors derived from them (`c600-run/tests/traces.rs`). The frozen test
  tariffs are `Tariffs::TEST`, labelled test-only.
- `c600-run/tests/det65_programs.rs` runs 11 of the 40 traces as programs through `Loaded::run`: nested-calls,
  equal-to-limit, step-precedence, simultaneous-exhaustion, simultaneous-memory-output, generator, the three display
  traces, output-equality and output-retained-at-cutoff. Each frozen row and each cutoff (dimension, phase, scope, denied row,
  source range) equals the vector, under LP3's limit mapping (`c600-runtime/tests/ct_rte_04.rs`): CT-DET-26, 27, 28
  (synthetic), 29, 31 and 32, all in part, on RT1 only.
- The other 29 are listed with the reason the IR draft cannot express them: child scopes (6), a generator started,
  aliased or returned in a call (4), a `let` released before its alias (2), text order (1), byte decoding (2), loops,
  assertions and flow checks (3), exact multiplication (3), streaming cursors (3) and display form 2 (5). They still
  replay through the meter.
- The product output profile is not the fixtures' (CV.5): a realisation publishes values with the fixture's byte
  counts, and `output-retained-at-cutoff`'s one-byte denied unit gets two bytes of kernel-cost headroom, since no
  `c600-json-1` unit is shorter than three bytes.
- `load` checks the IR identity and the program's names, arities, literals and precisions
  (`LG4:input.ir_version_unsupported`, `LG4:input.ir_rejected`). `Loaded::run` evaluates with an explicit work stack:
  one charge row per operation (DET.55), interruption observed before each operation (DET.56), aliases and owners
  (DET.58), each output unit as one fused row of encoding work, buffer and bytes (DET.56, DET.59), a preview
  request's priced `encode` pass before `invoke`, the generator `sha256-counter-draft-1` (DET.61) and `display_if`
  charging the selected branch only (DET.64). A stop names the call path, the operation and its source range.
- Values: exact `Int`, `Rat` and Q(√5) arithmetic, order and equality with widening, on `num-bigint`; `Approx<p>` for
  p in {24, 53, 113}, rounded to nearest with ties to even; canonical `c600-json-1` bytes with FP.6's exact and `f64`
  forms. Deep values compare, encode and drop without recursion.
- Drafts, which freeze with the IR (PHS.49): operation names, the stop locator, the frame layout (parameters,
  captures, `let` slots and operand temporaries; a body's own result has no slot), a value's canonical byte length as
  retained metadata (so an output unit is one fused row), the receipt request record, the JSON form of a word (LG2.18's
  word keys stay open) and `DraftTariffs`, test-only tariffs for the rows the frozen vectors do not price. The IR
  draft has one read construct for snapshots and workspace records; the IR freeze should split it.
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
- Drafts, which freeze with the IR and the command table: the acknowledgement is `{base, id}`, with the proposal
  number `<run>/<n>` as `id`; records are per run; the adapter serves a read of a name it stores as a workspace record
  and passes any other read to the engine; the engine's preview call does not take the block yet (KN5.9); the store is
  in memory and returns a stored value as written, and M6's session store decodes by the reader's type.
- Open: a built-in repair-list pick (CT-BND-29's third case) is refused with `PS1:capability.provenance_incompatible`
  until the session store resolves `c600-pick` records (M6). The command layer is LP3's `c600-command` (below); the
  boundary does not reach it yet.
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
  - Contract tests: CT-PS1-12 whole; the kernel halves (parts) of CT-LB2-02, CT-LB2-03, CT-LB2-04, CT-LG2-12 and
    CT-LG2-14; CT-KN2-07 as a part: a 100,001-letter Full word replays in chunks equal to its whole derivation. Not run:
    the recorded words of CT-KN2-07, which are jumbling words, and any Full stream beyond 3,000,000 letters in the test
    profile.

## Status of M5: the jumbling half

- `c600-jumble` is a candidate engine for the rigid jumbling puzzle under the state contract of 9 October 2026 with
  A1 to A4. It is built from `assets/` alone and reads nothing from the research reference J1 (PHS.18); its import
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
  with M4's halves; CT-JI-03. CT-KN2-02 in part: heights and domain IDs against `trees.py` and the library's
  histograms on H6030's compared states, the recorded fixture prefixes and u1, and the free-mode Cayley rotation.
- Not built, for M8: CT-KN1-03, CT-KN1-04, CT-KN1-06, CT-KN2-06, CT-KN2-07, CT-KN3-01, CT-KN3-03 to CT-KN3-06, R-11
  (CT-LG2-12), and CT-KN2-02's cases b20, b06-14, b14 and b03; the observations rim_summary, covers, repair_list, nu,
  checkpoints, release_check and derivation with reachability; Strict protection; and the holding caps of seam cells
  and tree positions past the first step.
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
- Not run, because M8 builds the calls: lead L9 (`label_checkpoint` on a raw table with a turned centre), L15's
  `repair_list` and `covers` rows (pair order, no rank, a missing cap), L2's `release_check` agreement and L16's
  `KN2:stale.revision_gone` (the candidate has no snapshot handles).
- Evidence kind: source and fixture. The comparison ran on Windows only. The timings the check prints are
  informational, not performance evidence.

## Status of M6: the session store

- `c600-session` keeps one session in one journal on a `Disk`: `FsDisk` over a directory, or `MemDisk` in memory for
  the tests. `MemDisk` fails a chosen write (an error, a partial write, or a write that persisted but reported
  failure), holds a write so a test can act while it is in flight, tears unsynced appends and loses unsynced data in
  a crash, and counts reads and writes.
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
  lead L8); there is
  no session lock, so a second opener could cut a frame that another opener acknowledged (Fable lead L9), and no
  historical workspace version (a snapshot of an earlier revision holds its state only); a proposal takes
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
  the undo chain), and L10's rows 1 to 7 are covered. L8 and L9 are deferred.
- Evidence kind: source and fixture. Failures are injected in memory; `FsDisk` runs in one reopen test, so a pass is no
  evidence of a real disk's behaviour in a crash. The checks ran on Windows only.

## Status of the evidence library: chapter 06

- `c600-evidence` holds chapter 06 as pure code. It reads and writes no file, clock, process or session (EVD.54), and a
  word is a function of the graph's facts and a pinned edition, `evidence-draft-1` (EVD.14). Where chapter 06 leaves an
  encoding or a detail open, the crate takes a draft, and its documentation (`c600-evidence/src/lib.rs`) lists each
  one; none is a decision of the chapter. The trial threshold stays the owner's (EVD.29), so the tests use the trial
  set and its up-subsets only.
- Tests (`c600-evidence/tests/`): `words.rs` runs the word function's rows and checks (CT-EVD-01, CT-EVD-05,
  CT-EVD-07 to CT-EVD-11, CT-EVD-13, CT-EVD-22); `properties.rs` runs seeded random graphs (CT-EVD-02 to CT-EVD-04,
  CT-EVD-06); `sinks.rs` runs writers, reuse, check reports and the import rules (CT-EVD-17 to CT-EVD-21, CT-EVD-26 to
  CT-EVD-30, and the evidence halves of CT-LB6-02 and CT-PS1-27); `examples.rs` transcribes section 9's E1 to E3
  (part of CT-EVD-12).
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
    `vectors.rs`, and the built-in column's `built_in.rs`, M7 attack leads' `leads.rs` and M1 limits' `limits.rs`).
  - `c600-syntax/tests/`: `conformance_lg1.rs` to `conformance_lg3.rs` write the LG tests in source; `errors.rs` holds
    CT-ERR-05's encoding half (ERR.10's raw context for each malformed shape); `syntax.rs`, `format.rs` and
    `lowering.rs` mark M7's tests.
  - `c600-run/tests/`: `conformance_lg2.rs` to `conformance_lg4.rs` run the LG tests (shared helpers in
    `conformance/mod.rs`); `conformance_bnd.rs` runs the boundary tests, and `tables.rs` enumerates the grant table
    and the corpus; `legacy.rs` runs CT-PS1-18's legacy parts; `conformance_det.rs` runs the CT-DET rows on RT1;
    `canonical_ps1.rs` runs the canonical-profile rows (CT-PS1-01 to CT-PS1-07, CT-DET-23 and, synthetic, CT-KN3-02);
    `traces.rs` and `values.rs` mark the meter's and the values' rows; `det65_programs.rs` runs 11 of DET.65's 40
    traces as programs and lists why the other 29 do not run.
  - `c600-kernel/tests/`: `kernel.rs`, `load.rs` and `json.rs` run the 600-cell-Full halves of the KN rows, CT-PS1-04
    and CT-PS1-12 (M4's section).
  - `c600-jumble/tests/`: `crossing.rs` runs CT-KN2-03, and `assets.rs` the jumbling model's asset checks (M5's
    section); `domain.rs` and `refusals.rs` hold the candidate's halves of Fable's M5 attack leads.
  - `c600-evidence/tests/` runs the EVD rows (the evidence library's section).
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
    faults as expected failures (M5's section).
- Count with LP3's C1 to C5 and C7, M5 and LP2's first integration batch, as `tools/language/ct_index.py --summary`
  prints it
  (`tests/test_ct_index.py` checks the tool): 59 whole, seven of them by the union of their parts (CT-KN1-05,
  CT-KN2-01, CT-KN2-04, CT-KN2-05, CT-LG2-04, CT-LG3-01 and CT-LG3-04); 118 part; 5 named only by LP3's command-layer
  tests, as parts without a marker (CT-KN5-07, CT-KN5-12, CT-LI-04, CT-LI-09 and CT-LI-10); 1 missing despite its name
  (CT-KN5-08: only the documentation in `c600-command/src/layer.rs` names it); 65 waiting, each with what it waits for
  in `docs/progress/1.0/lp2-ct-index-notes.md`.
- Not run or not applicable, which is no pass (PHS.63), with the check that runs instead: CT-LG3-06's flow-label
  variant, while the flow rule is off (BND.41) and LQ2 has not set it, so the static-only and run-time variants run
  (`c600-run/tests/conformance_lg3.rs`); CT-LG1-03's second host context, since CI runs on Windows only and the editor
  services that host the formatter come with LP4; CT-LG4-01's second admitted route, since RT1 is the only one (LQ9);
  CT-BND-11's ONE.32 fields in other profiles, which never arise (BND.60); the Linux legs of CT-DET-02, CT-DET-15 and
  CT-PS1-01, since CI has no Linux cargo job; form 2 in CT-DET-32, since RT1 runs form 1 only (the LP2 plan's
  selection, which the owner may overrule); ERR.1 for the five codes `c600-command` names as drafts that chapter 13
  does not register, which `tests/test_language_scans.py` lists exactly (`UNREGISTERED_DRAFTS`); the parts of the
  session's rows that their markers leave to LP3's view, display, command layer and runtime process, to A6's live
  twist and to M5's crossing, and step 7 of CT-ONE-11, which M6 cannot reach (M6's section); and, as before,
  CT-LG1-01's mixed dependency editions and CT-LG2-05's form 2.
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
- Open, each needing a decision:
  - Exact sinks take any value: the checker admits `use_evidence(f)` with a function value, and the interpreter passes
    the value to the host's exact sink without encoding it, so a function or a receipt reaches the host, against RPI.54
    and RPI.1. The frozen text does not say what a goal or evidence sink accepts.
  - ERR.6: `c600-kernel/tests/kernel.rs` compares refusal texts, because `c600_kernel::Refusal` has no typed context
    (CT-ERR-16's typed failed fact); `c600_syntax::LowerError` has no code.
  - For the owner under PHS.52 (frozen text): in `13-errors.md`, the LB6 closure row `evidence.decomposition_mismatch`
    (line 661) sits between ERR.28's table and ERR.31 and belongs at the head of ERR.31's table, and ERR.31 comes
    before ERR.29 and ERR.30; the rule source of `KN5:stale.filter_preview` (line 322) omits KN0.9
    (`closure/manifests/p7-layers.json`). The Used-by cells of `KN2:input.recipe_shape` and
    `KN2:resource_limit.primitive_ceiling` say "not yet used", but the kernel emits both; the cell of
    `KN2:input.word_blocked_prefix` says the same, but the jumbling adapter emits it for a refused word (M5).
    Chapter 13 registers none of the five draft codes `c600-command` emits (`KN2:consistency.star_certificate`,
    `KN5:input.unknown_row`, `KN5:input.argument_form`, `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict`);
    the command table's drafts settle at LP3 (PHS.53).
  - For LP3: LB7.1's `LB7:capability.module_grant_missing` is the workspace library's entry-admission fact, built with
    LB7; the checker's own admission reports `LG3:capability.not_granted`, as chapter 02's LG3 table has it.
  - KN5 has two implementations, `c600-session` (M6) and LP3's `c600-command` (pull request 159). The LP2 integrator
    picks the one CT-KN5-01 to CT-KN5-13 run against before M8.
  - `Kernel::protection` has no model check (M4's section).
  - For M6's store: the kernel's `archive::gunzip` accepts concatenated gzip members and trailing zero padding, so
    FP.7's strict one-member reader must refuse them itself; run-time `encode` applies no json_depth or integer_digits
    limit, so the store's writer must check FP.9 before it persists a value.
  - Gaps: the interpreter does not check a function's kind at run time; `provenance::import` makes the author `own`,
    so `Author::Imported` never arises (FP.18's example has `own`); `Loaded::run` does not require the entry function
    to be exported, and `load` does not run the checker again.
- Missing, by what they wait for:
  - LP3's lanes, views and shell (16): CT-ONE-13 to CT-ONE-16, CT-LI-01 to CT-LI-03, CT-LI-05 to CT-LI-07,
    CT-LI-11 to CT-LI-14, CT-EV4-03 and CT-PS1-13.
  - M6's remaining rows (15): CT-PS1-08 to CT-PS1-10, CT-PS1-16, CT-PS1-19, CT-PS1-21, CT-PS1-23, CT-PS2-01 to
    CT-PS2-05, and the run records of CT-DET-04, CT-DET-14 and CT-BND-23 (with LP3's grants for CT-DET-04 and
    CT-BND-23).
  - The jumbled puzzle's rows that M5 leaves to M8 (9): CT-KN1-03, CT-KN1-06, CT-KN2-06, CT-KN3-01, CT-KN3-03,
    CT-LG2-12, CT-PS1-11, CT-PS1-14 and CT-PS1-22.
  - A session's second level (3): CT-ONE-17 and CT-ONE-18, which need a second level in the session, its crossing
    and `project` (KN1.9); and CT-ONE-19, with LP3's view model.
  - Derived states with an open reachability claim and KN2.14's raw-table import, which M4 does not build (3, no
    milestone named): CT-ONE-09, CT-ONE-10 and CT-ONE-28.
  - LG5's module loader and lockfiles (7): CT-LG5-01 to CT-LG5-07. LP3 C4's `c600-module` builds closures and
    lockfiles, but its tests name none of these rows.
  - KN5's command layer, once its candidate is picked (2): CT-KN5-05 and CT-KN5-08.
  - A6's exact rotation requests (5, no milestone named): CT-KN3-04 to CT-KN3-06, CT-LG2-16 and CT-DET-16.
  - The stage checker, RPI.68 to RPI.76 (3, no milestone named): CT-LG2-15, CT-LG3-10 and CT-LG5-08.
  - Streaming and compressed words, KN2.10 (no milestone named): CT-LG2-14 and CT-LG4-03, and CT-KN2-07, named as not
    built.
  - A builder and its build cache (2, no milestone named): CT-DET-21 and CT-DET-22.
  - A pointwise or whole-state contract form in the IR (no milestone named): CT-LG4-05, named as not built.
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
- Drafts, which freeze with the command table at LP3 (PHS.53): the list in the crate documentation, among them one
  session per layer with Jumble off, an inline recipe as `WordRef`, `twist.apply` refused until KN3's exact rotations
  exist, five codes chapter 13 does not register yet, and the envelope's epoch (LI.5).
- Not built: the durable transaction is `MemoryStore`, a stand-in until M6 is on main, so no receipt here is
  persistence evidence; the source route, the views and the other two tools (LP3 plan, C2 to C8); protection rows;
  every row the crate refuses with `KN5:input.row_not_built`.
- Critical-path read (`AGENTS.md`, Codex minimum): a fresh Claude subagent on a smaller model read the crate with the
  acceptance check and found no blocker. Fixed before the pull request: R1 (major), a retry after a restart was refused
  as `command_id_reused` because the request digest used the layer's current epoch, not the envelope's; R2 (minor),
  the token's binding was checked before commit-lane admission; R3 (nit), the outcome lookup ignored the request
  digest. A scoped re-check found all three fixed and no new blocking finding.
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
- Drafts, which freeze with the IR and the command table: the list in the crate documentation, among them Global and
  Local's fixed IDs, `LG3:capability.not_granted` for a recomputation that names a view row (RPI.83 names no code), a
  separate command-ID namespace for scheduled recomputations (LI.25), the status precedence (RPI.13, charter D2), and an
  LEB128 byte form standing in for H8's canonical output bytes. C2's next view piece adds one view model per session
  profile, which no rule reads yet (LG3.9, RPI.42); a provenance block the host fills for installed output (RPI.45,
  BND.22); the built-in class declared when a view opens (BND.27); the chart evaluator as the view's declared chain
  (RPI.28, RPI.68); rebuild counters per view; a command ID on `view.open`, with scheduled IDs refused in both view
  rows (RPI.83, LI.25); and engine epochs in revisions, bundle digests and the publication bytes (now version 2): a
  bundle from an older epoch is refused with `refresh_required`, and output attaches only within its own epoch
  (KN0.2, ONE.60, ONE.63).
- Not built in the views: CT-RPI-26's gate half (charter E2 and the R-19 captures) and CT-EV3-08's measured half
  (RM.39's measured ceilings) wait for C9; LQ2's policy table and display refusal wait for the joint setting.
- `c600-display` is the host side of chapter 10's display plans and RM's feature matrix without a renderer. The
  validator checks RPI.10, RM.11, RM.12 and RM.20 in the order invalid, unsupported, ceiling. A publication installs
  whole or not at all and ends in exactly one KN5.21-style terminal, standalone or scheduled (RM.17 to RM.26, LI.25);
  a mock backend stands in for an adapter (RPI.55, RM.33). Picks resolve by instance ID through the installed source
  map, never through a chart inverse, and stale picks name the field that changed (RM.10, RM.24, RPI.24, RPI.25).
- Charts and stages (C2's next display piece): exact arithmetic in Q(√5) with checked `i128` parts, the 600-cell and
  its exact poses in K⁺ (`exact.rs`); chart checks with the exact region test, the vertex test and `admit_chart`, which
  admits a chart only on established check results and refuses it on a refutation with
  `EV3:evidence.chart_check_failed` (`chart.rs`); a binary32 CPU reference evaluator and RPI.56's key over the exact
  fields (`reference.rs`); and the stage output rules and their composition order (`stage.rs`).
- Display tests (`c600-display/tests/`): CT-RPI-02, CT-RPI-04, CT-RPI-05, CT-RPI-09, CT-RPI-10, CT-RPI-11,
  CT-RPI-27, CT-RM-03 and CT-RM-04; CT-RPI-08's 600-cell-Full rows (F01 to F17, with J02 and J04 unsupported on
  Full); the stale half of CT-RPI-12; RM.37's fifteen rows of CT-RM-07 as fixture payloads over the Full geometry
  digests. C2's next display piece adds CT-RPI-14 over the drafted chart repertoire (RPI.34 is open), CT-RPI-18 on
  the CPU reference (its device version, CT-RPI-33, waits for the renderer), CT-RPI-19's Windows leg, CT-RPI-22's
  chart half (with the views' limit half, CT-RPI-22 runs whole) and the headless part of CT-RPI-32.
- Display drafts: the list in the crate documentation, among them the validation order, string dependency keys,
  set digests over sorted piece IDs, canonical JSON standing in for the output bytes, form `stop` for every error
  after acceptance, and a publication to a view that is not open being invalid. C2's next display piece adds the
  drafted chart repertoire (stereographic charts declared by their pole, over convex hulls of exact vertices, with the
  properties domain, injective and section; RPI.34), exact poses only (every float pose is refused, never rounded;
  RPI.30, LB4.2), Q(√5) arithmetic local to the crate (KN3.2, KN3.3), the fixtures' own precision rule (1e-4 absolute,
  with an exclusion of 0.05 per vertex; RPI.36), RPI.56's key over the exact fields only, and the stage output rules
  as diagnostics (RPI.68, RPI.69, RPI.75). C2's draft that every exact chart property is `not_established` now holds
  only when the validator's context carries no check result for it (RPI.48).
- Not built: CT-RPI-19's Linux leg (no Linux run in this session); the rest of CT-RPI-32, which needs the stage
  module's parser and checker (RPI.69 to RPI.71, RPI.77, RM.38's weight table) and a backend for HLSL compilation, the
  raster bound and frame reports (RPI.72, RPI.78, RM.41); KN3's polytope operations in the kernel, so the region test
  is local to this crate; charts and properties outside the drafted repertoire, the GPU agreement check (RPI.35) and a
  production precision rule (RPI.36). CT-RPI-08's jumbling rows and the posed producer wait for M5; a decoder from the
  interpreter's values needs list types in the IR (LP2); F05 and F26 stay unsupported until the stage checker exists;
  CT-RM-09's rows 3 to 6, 8 and 10 (the raster bound, frame-report stops and device-fault quarantine) need a stage
  checker or a device; RM.38's weight table, device loss and `display_unsupported` belong to the renderer and its
  backend.
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
- Tests (`c600-module/tests/`): CT-RTE-08's source leg (`ct_rte_08.rs`, 13 tests): the approval binds every
  executed module; substituted, missing and mismatched dependencies are refused; record forms are checked before
  use; a moved file keeps its closure and its approval; dynamic code is refused; capability separation holds at the
  preview point. CT-RTE-07 on 600-cell-Full (`ct_rte_07_full.rs`): 36 of LX-3's 75 frozen keys, each run ten times
  with one SHA-256 over its canonical result, engine side and store, and a check that every key is either realised
  or listed as left, exactly once.
- Differences from the corpus, listed in the tests: A25 is a false refusal, which the corpus permits (separation
  refuses the preview of an allowed combination). P01 to P03 give `LG3:capability.not_granted` where the corpus
  expects `LG3:capability.ambient_read`, which chapter 13 does not yet list as a concrete code; the code is the
  checker's (LP2).
- Drafts, which freeze with the module formats (LG5.10, LQ10): the list in the crate documentation, among them the
  manifest and lock forms (`c600-module-draft` and `c600-lock-draft`, version 1), a null FP.11 identity for both
  records with the model identities in `payload.supports` (a gap for the format review), locations in the lock
  only, the closure digest, the in-memory approval record `c600-approval-draft`, the nearest code for dynamic code,
  and the separation refusal at the preview point, which stays in this crate until C5's host bridge.
- Not built: CT-RTE-08's persisted words (C7, on M6's store). CT-RTE-07's other 39 keys: 6 jumbling keys (M5); 6
  keys that need the session store (C7); 15 keys the IR draft cannot express (early stop, word length, inverse and
  concatenation, exact multiplication and remainder, and a run's input as a query argument; LP2); and 12 budget
  keys, whose amounts follow LX-3's experiment-only counting fixture rather than `lp1-exec-draft-1`'s prices, or
  which need host events inside a loop, the host's row cache or the command layer's rows. The Linux leg did not
  run.
- Evidence kind: source and synthetic. No timing is measured.

## Status of LP3, C3: the runtime in its own process

- `c600-runtime` runs a program in a child process of its own (route RT1, LQ9; RTE.21, ONE.69). The child binary
  reads framed messages on standard input, compiles and runs each job with `c600-syntax`, `c600-check` and
  `c600-run`, and forwards every host call (`tariff`, `perform`, `sink`) to the parent. The parent starts the child,
  answers its calls from the caller's host and returns the run's end or a typed fault. `run_in_process` is the same
  path in one process, the reference the tests hold the child to.
- Protocol draft `c600-rt1-wire-1`: length-prefixed frames of at most 16 MiB either way, one protocol byte, six
  message kinds, and a canonical value encoding that refuses every noncanonical form on both sides. The parent
  answers a repeated, unacknowledged call from its record and never calls the host twice for one call, and a new
  call must acknowledge the last answer, so at most one answer is open (KI.5). No message has a field for a token
  or any other authority (KN5.15).
- Faults: a child that is lost, does not start, names another protocol, sends a malformed or oversized frame or
  breaks the call protocol ends the run as `error` with `KN7:resource_limit.runtime_lost`; a passed deadline ends it
  as `cut_off` with `KN7:resource_limit.safety_stop`. The child is killed and reaped, and the next run starts a fresh
  one.
- Tests (`c600-runtime/tests/`): the child and the in-process run give the same results, charges, stop points, host
  calls and outputs (`equivalence.rs`); CT-RTE-04 in part, 8 of DET.65's 40 traces as `.6cs` programs on both
  executors (`ct_rte_04.rs`); the protocol's bounds, canonical encodings, KI.5's record and KN5.15's field list
  (`protocol.rs`); CT-RTE-10's headless fault half, rehearsals in debug builds that kill, crash, truncate, garble,
  oversize or hang the child while the session keeps its journal, state hash and pending preview (`faults.rs`).
- Drafts, which freeze with the IR and the command table: the list in the crate documentation, among them the
  framing and message kinds (KI.15, ONE.69), the transfer encoding (DET.59), acknowledgement and retry (KI.5), the
  fault codes (KN7.13), source bytes rather than checked IR in a job (KI.6, ADR-L12.9), and a deadline checked
  between messages only, so a blocking host call is not interrupted.
- Not built: OS isolation of the child (a job object with kill-on-close, a restricted or AppContainer token,
  filesystem and network confinement); a bound on the parent's queue of frames to a child that stops reading its
  input (draft 5); LX-2's and LX-4's decisive runs (C9, on the owner's machine). CT-RTE-04's other 32 traces need
  what the IR draft lacks: child scopes, more than one generator per run, releasing a `let` before its alias, text
  order, byte decoding, loops and pointwise checks, exact multiplication, streaming cursors, one-row output units,
  and an executor for `lp1-exec-region-draft-1` (LP2). Two of them, `nested-calls` and the output rows, may be meter
  mismatches in `c600-run`; LP2 has them.
- Evidence kind: source and synthetic. The fault rehearsals are not LX-2 evidence, and no timing is measured.

## Status of LP3, C5: the source route

- `c600-tools` holds the built-in 600-cell-Full tool as 600 Cell Script source,
  [`full_600_cell.6cs`](c600-tools/src/full_600_cell.6cs), in edition `6cs-draft-1` (syntax candidate A). Its one
  entry, the exported command `preview_word(w: Word)`, proposes the word the solver named to `operation.preview` and
  returns the acknowledgement's two fields as a record `{base, id}`. The IR draft has no list types and no word
  operations, so the library entries of chapter 03 (LB1 to LB4) wait for the IR.
- Admission: the checker admits the source with no diagnostic. The entry's only effect is `preview` and it requests no
  capability, so it prepares previews and reads no state (LG3.13, BND.39). `c600-check`'s `Profile::Solving` is the
  column of a solver's own tool, which prepares no preview until the joint setting; `tool::admit_built_in` applies the
  built-in column of chapter 07 section 8 over the checker's report. A variant that also reads state is refused with
  `LG3:capability.not_granted`. The class (`analysis`, `checking`, after LB6.5) and the row set are declared in the
  crate's Rust manifest, because the IR and the syntax carry no module manifest yet.
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
  `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}` and no token. The program still receives only the fixed
  acknowledgement; the layer's receipt goes to the shell. The bridge grants no state read, declassified value, file or
  workspace access, view, exact sink or clock. The command layer's own rules refuse a runtime invocation of a commit or
  app row and any runtime invocation carrying a token (ADR-L02, KN5.15), and only a token the shell mints commits the
  pending preview the runtime proposed.
- The source route: the binary `c600-lp3-source` replays O01 to O07. Per step the runner expands the case recipe to
  its literal word, the tool's program runs on that word and proposes it through the bridge, and the shell presents
  the preview, commits it with the commit chord `Ctrl+Shift+Enter`, and undoes and redoes on the command route's
  schedule. `python tests/test_lp3_source_route.py` builds `c600-lp3-oracle` and `c600-lp3-source` and passed with:
  - `tools/oracle/compare.py`: all seven cases equal on the source route, 3,201 steps, no omitted field;
  - every source-route trace byte-identical to `c600-lp3-oracle --route command`'s;
  - the two routes' receipts agreeing on status, job, error code and revision after in all 13,553 pairs, and differing
    only in command ID, caller and request digest; the 289 preview-ready result digests agree once rebased onto the
    case recipe's draft, and the six O07 recipes with no literal word are the kernel's refusals on both routes;
  - a negative control: negating the first letter of O06 step 3's literal word before the tool runs makes compare.py
    fail at that step (`expansion_sha256`).

  CI runs this check through `tools/ci/run_headless.py`.
- Tests (`c600-tools/tests/`), each naming the conformance parts it covers; no conformance ID runs whole:
  - `tool.rs`: the source parses and lowers; the checker admits it and it only prepares previews (CT-LB6-03, part); a
    variant that also reads state is refused (LQ21; CT-LB6-03, part); a built-in previews only the word the solver
    named; a refused built-in, or a module with one checker diagnostic, never runs and never calls the engine, and a
    word from state relayed through a workspace record or a declassifier is refused (CT-LB6-03, part); the class and
    the rows are declared (CT-BND-15, part).
  - `ack.rs`: the fixed acknowledgement against an engine double (CT-BND-11 and CT-BND-07, parts).
  - `bridge.rs`: one runtime preview per proposal and the fixed acknowledgement on the command layer (CT-BND-11 and
    CT-BND-07, parts); no commit or app row for the runtime (ADR-L02, KN5.15); a runtime invocation carrying a token is
    refused (KN5.15); only a token the shell mints commits the pending preview (CT-ONE-29, part, headless); the bridge
    grants nothing but a proposal.
  - `expand.rs`: every case step expands to its recorded literal word, and the expansion is the kernel's literal.
- Open: the built-in column lives in this crate because `c600-check` has no author or column parameter on `admit`; it
  does not judge a reading entry's workspace writes against the class's rows (BND.57). The command layer's receipts
  carry no program digest; the proposal's provenance block names the digest of the source bytes that ran.
- Drafts, which freeze with the IR and the command table, are in the crate documentation under "Drafts".
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
  - `repeat.rs`: CT-EV4-04's append, preview and commit legs and its busy leg, with repeat events flagged and
    unflagged, counting the invocations that reach the layer. The twist leg waits for KN3.7's exact rotations (LP2).
  - `keymap.rs`: CT-EV4-06's synthetic half, ahead of LP4 (chapter 12 section 15.5), among them DT-M1's US and French
    layout counterexample.
- Drafts, which freeze with the command table at LP3 (PHS.53) and chapter 05's key-map forms: the list in the crate
  documentation, among them the routing class names, the chord form (W3C `code` names with Ctrl, Shift, Alt and
  Meta), the two layout tables, the default global map, the repeat flag's source, and placeholder camera and twist
  values. Escape closes a pending prefix group or the innermost transient layer and never cancels a preview (DT-M1 C4).
- Open:
  - The class rule (no text-class or capture-class binding; list-class bindings reach only the list rows) reports
    `EV4:input.binding_conflict`, since chapter 13 has no row of its own for it; a dedicated row is a change to
    chapter 13 (PHS.52).
  - Two draft key maps exist: the shell's five chords in `c600-command`, which C1's key route uses, and the router's
    default map. They agree on preview, execute, undo and redo; the shell binds Escape to `operation.preview.cancel`,
    which DT-M1 C4 rules out. One map remains when the command table freezes (PHS.53).
- Evidence kind: source and synthetic. No Windows, IME, device or timing claim; CT-EV4-05 stays a windows test (LP4).

## Status of LP3, C7: durable receipts on the session store

- KN5 gets one implementation, owned by LP2 under PHS.18 (integrator, 16 November 2026). The LP2 integrator placed it
  in `c600-command`, which keeps its `Store` trait, with `c600-session` (KN4) implementing the trait, and M6's
  semantics hold where the frozen text leaves a choice; LP2 writes it on branch `claude/lp2-kn5-one`. Until it lands,
  LP3 changes neither `c600-command`'s layer, token, receipt, store and shell code nor `c600-session`, and
  `MemoryStore` stays the layer's stand-in. C7's first piece is therefore `c600-durable`, a test-only crate that
  drives M6's session store and C4's module closures through their public APIs and changes neither them nor
  `c600-command`.
- Tests (`c600-durable/tests/`):
  - `ct_rte_08_persisted_word.rs`: CT-RTE-08's persisted-word case, its durable leg (BND.22, BND.84, BND.85, LG5.1,
    LG5.6). An approved own tool stores a word, and the shell persists it through M6's `workspace.set` with the
    boundary's provenance block. After a power loss on a memory disk, and in a new session on a file-system disk in
    a fresh folder under Cargo's test directory, the record and its stored receipt come back equal (KN5.23); the
    block names the module the approval binds and no other; preview admission refuses it; and a solving run that
    proposes it is refused at admission, with nothing reaching the engine.
  - `kn5_on_session.rs`: CT-EV3-07's durable action order (a commit, a `workspace.set` and a commit take orders 1 to
    3, an undo hint taken before the set is refused, the order and all three receipts survive a crash, and undo runs
    3, 2, 1) and its view part on M6's stand-ins (a query job, an opened snapshot and a preview leave the order, the
    undo hints, the revision and the journal bytes unchanged, and a program's or a job's workspace write is
    refused); CT-LB2-03's work-sheet part (setting a named sheet again makes the old preview stale, and a new
    proposal reads the new content, also after a reopen).
- Drafts, in the crate documentation: the shell's `workspace.set` as the host's delivery of a stored word (C5's host
  bridge decides the real path); reading a record back through `c600_session::journal::scan`, since M6 has no record
  reader; M6's stand-ins for views; and `c600_run::boundary::admit_preview` standing in for KN5.26.
- Waits for LP2's KN5 implementation: durable receipts and undo through the command layer, KN5.26's admission of
  record words, EV3.2's view rows, LB7.2's work-sheet structure and a session lock. If it changes an API these
  tests use, its branch ports them.
- Evidence kind: source and fixture. The file-system leg is a short test in a fresh folder, not long-session
  evidence. The checks ran on Windows only; the Linux leg was not run.

## Status of LP3, C5 next: the process host and the tool's views

- The process host, `c600_tools::process::ProcessHost`, runs the 600-cell-Full tool in `c600-runtime`'s child (C3; route
  RT1, RTE.21). The child gets the source bytes, the manifest's module and entry, the literal word and the limits, and
  no token, secret, grant or command layer; the boundary and the bridge stay in the caller's process as the runtime's
  host, so the program still receives only the fixed acknowledgement. One child serves every run, and after a fault the
  next run starts a fresh one (KN7.13). The child admits the job as tool authoring and the built-in column stays the
  caller's, because `c600_check::admit` has no author or column parameter.
- `c600-lp3-source --host process` replays O01 to O07, and `python tests/test_lp3_source_route.py` runs both hosts:
  compare.py equality on all seven cases, traces byte-identical across the hosts and to the command route's, and step
  records, receipts and callers equal across the hosts. Four O07 steps (0, 1, 2 and 8) have a letter outside 1 to 1200
  in magnitude, which the runtime's wire cannot encode: on the process host the job is refused at the start stage
  (`PS1:input.noncanonical_value`) and has no receipt, and the step record keeps the kernel's refusal, which the check
  compares with the in-process receipt's error.
- The tool's views: `c600_tools::views` is host code, since the tool publishes no view (LQ21, LG3.13, charter E6). It
  builds the display contract's publication of a proposal and its preview on `c600-display`, the content the headless
  view model installs on `c600-view`, and the presentation report to the shell, which calls `present` only for a frame
  the built-in chain drew.
- Tests (`c600-tools/tests/`), each naming its part:
  - `views.rs`: CT-EV3-01, CT-EV3-02 and CT-EV3-04, whole for the tool's view; CT-EV3-03, whole, its imported half on a
    fixture import record; CT-BND-19 for the tool's views (node text, derivation, derived subject, preview literal and
    installed bytes hold only the word the solver named); CT-ONE-29's user-stage part at the adapter (a user-stage frame
    mints no token and the commit chord finds no fresh preview; a built-in-chain frame's token commits).
  - `process.rs`: the same summaries, values, shell records, invocations, receipts and layer on both hosts; the job
    carries only the admitted source; a word with no transfer encoding is refused at the start stage; a runtime that
    does not start is reported and changes nothing; the binary names the host in every record (CT-BND-11 and CT-BND-07,
    parts).
  - `net_oversize.rs`, on both hosts: CT-LB2-02's Net half (an identity word whose prefixes move protected orbits
    previews clear in Net mode and commits as one transaction, and the prefix alone is refused); CT-LB2-03's oversize
    refusal (100,001 letters refused whole on both routes with the same code, 100,000 admitted) and its chunking part on
    the command route.
  - `lb3_checker.rs`: CT-LB3-07's checker half, as far as capability separation decides it (an outcome table that reads
    state and prepares previews is refused; a named-word check that answers only the acknowledgement is admitted).
  - `kn5_retry_lead.rs`: a lead on KN5.21 to KN5.23, confirmed. A refused envelope that reuses a genuine
    `operation.execute` envelope's command ID, arguments and binding (a token minted with another secret, or a runtime
    caller) is stored as that command ID's outcome, so the genuine envelope never commits under it; a fresh command ID
    still commits. The test records today's behaviour; the requirement went to LP2's KN5 implementation.
- Not built: the shell's own refusal of a user-stage frame (LP2's KN5 implementation adds a presented-frame input to
  `present`); CT-BND-19's live-twist part (KN3.7's exact rotations); an import row; grip, cell or piece objects for a
  word's letters (the kernel has no letter-to-grip map); a view declaring both of the manifest's classes (`c600-view`'s
  `Declared` holds one class); receipts on the process host for words with no transfer encoding; CT-LB2-02's Strict half
  (the kernel has Net only); named work-sheet reuse; chunked recipes over 100,000 letters on the source route (the
  tool's entry takes one flat word); CT-LB3-01's checker half and CT-LB3-07's recommendation rule (`c600-check` decides
  neither); the child's own built-in admission.
- Drafts, which freeze with the IR and the command table, are in the crate documentation under "Drafts".
- Evidence kind: source and synthetic. No Windows, DirectX, input or timing claim; the source-route check's timings are
  informational. The checks ran on Windows only; the Linux leg was not run.

## Status of LP3, C6: the representations tool, headless half

- `c600-repr` is LB6.5's row "Representation, work-set and filter tools", class `view_aid`, over LB4's layouts and
  atlases, map property checks and linked selection. It draws nothing and owns no device, renderer, clock or session:
  picks resolve through `c600-display`'s host and the installed source map, exact chart checks are `c600-display`'s, and
  each entry's admission is `c600_tools::tool::admit_built_in`.
  - `layout`: sheets and atlases as display plans with their source maps; a layout filter rebuilds the layout, the
    instance list and the map's entries and leaves the mechanically analysed set alone (LB4.4); a linked selection is
    computed once for its inputs, and frames only read it (EV3.10).
  - `chart`: a chart's survey over camera poses, with exact verdicts at exact poses from `c600-display`'s checks and
    display data between them (LB4.2), and a user chart's admission as far as `c600-display` reaches it (LB4.3).
  - `tool`: the source [`representations.6cs`](c600-repr/src/representations.6cs) (edition `6cs-draft-1`, syntax
    candidate A) with five entries, each admitted under the built-in column; the Rust manifest with the class
    `view_aid`; the review of the class against the exact entry definitions and the row set, which also finds an
    exported function the manifest does not list and source bytes other than the ones the manifest names; and the form
    of a review record that names every entry digest (BND.37).
  - `rows`: what each of the six rows needs from the command layer, which refuses them as not built: a dispatch from a
    stored row to a built-in tool's entry, the runtime's view and workspace sinks as the job's result, argument forms
    for the rows' reference types, and a workspace revision that moves. `set.save` also needs a list type in the IR
    draft. LP3 changes no layer code until KN5's one implementation lands.
- Tests (`c600-repr/tests/`), each naming its part:
  - `ct_lb4_03.rs`: CT-LB4-03's 600-cell-Full half. Display instances of two charts map through the installed source map
    to occurrence, sticker, piece, cell and slot; a chart's inverse value is never consulted for a pick; a layout filter
    preserves the analysed set. Pose levels are C8's next piece, on M5's kernel.
  - `ct_lb4_charts.rs`: CT-LB4-01's CPU reference (a declared domain decided by an exact region test and section
    membership on the posed vertices, boundary cases included; a chart whose vertices pass and whose domain fails inside
    is refused; an established domain establishes no injectivity); CT-LB4-02, whole (synthetic: replacing the decisions
    between exact poses changes only display data); CT-LB4-04's admission half and CPU half.
  - `tool.rs`: the source parses and lowers, and every entry is admitted; CT-BND-15's representations part; CT-LB6-03's
    representations part (a stash-and-relay, a state-chosen target, a writing view and an exported function the manifest
    does not list are each found). The record is checked with a fixture reviewer: BM5's review of the tool happens when
    it ships.
  - `ct_ev3.rs`: CT-EV3-06's counter half (unchanged frame draws and a workspace-only revision compute no relation;
    counted, never timed); CT-EV3-07's view half at the command layer (the checker refuses a view that writes the
    workspace or proposes; after a commit, program calls to the six rows and to `session.undo` are refused, a view
    overlay row completes as `view_published`, and the revision, the log and the undo hint stay as they were).
  - `rows.rs`: the requirements name the stored rows as H-08 has them, each row's effects are its entry's, and the layer
    refuses all six rows; an overlay cannot replace one.
- Not built: the six rows and a `set.save` entry; the checkers for homomorphism, embedding, isometry and
  full-dimensional injectivity (CT-LB4-01's rest, which needs KN3's exact polytope operations); the stage checker, HLSL
  compilation and every device step (CT-LB4-04's rest, the renderer freeze); pose levels and off-lattice relations (C8's
  next piece, on M5's kernel); the relationship strip; filters of trees, complexes and graphs; a BM5 review record; an
  explicit workspace command through the command layer (KN5's one implementation).
- Drafts, which freeze with the IR and the command table, are in the crate documentation under "Drafts".
- Evidence kind: source and synthetic. No Windows, DirectX, input or timing claim. The checks ran on Windows only; the
  Linux leg was not run.

## Status of LP3, C8: the jumbling tool, headless half

- `c600-jumble-tool` is LB6.5's row "Named twist/word checker and kept checks", class `checking`: PHS.22's built-in tool
  for jumbling. Its source [`jumble_checker.6cs`](c600-jumble-tool/src/jumble_checker.6cs) (edition `6cs-draft-1`,
  syntax candidate A) has two entries, `check_named_twist` and `check_named_word`, each admitted under the built-in
  column through `c600_tools::tool`. Each proposes the literal word the solver named and reads no state, so the tool
  never chooses or outputs a word by itself. `word` reads a `Word` as literal jumbling letters, cap then item, of the
  state's menu; a word that is no sequence of letters is refused before any letter is applied.
- `c600-display` gains the jumbling producers (`jumble`): the posed-sticker row J02 from a state's canonical pose table,
  the certified transform J03, the certified overlays J04 (the full named-cap set, the cap-map witness and the named
  cut), J08's seam table from the engine's `seam_facts`, and the slot projection J13, with a fixture support report for
  J02 to J05. Under a jumbling identity the validator now checks a posed row instead of refusing it; 600-cell-Full is
  unchanged.
- Tests, each naming its part:
  - `c600-jumble-tool/tests/tool.rs`: CT-LB6-03's jumbling part (the declared class against the exact entry definitions;
    LX-3's correction-solve, state-based target and storage-laundering cases refused under capability separation and
    never run) and CT-BND-15's jumbling part (the class, the row set `operation.preview` and a manifest that names the
    source digest).
  - `c600-jumble-tool/tests/lb3.rs`: the kernel halves of CT-LB1-02 (at a blocked cap literal a,a⁻¹ is refused at its
    first letter and the empty word is admitted; prefix protection reads literal letters), CT-LB3-03 (heights (0,0) off
    the lattice in free mode; domain IDs by exact coset tests), CT-LB3-06 (prefix peak, excursion, tameness and the step
    flag recomputed apart from the kernel's batch; a false flag refuses nothing) and CT-LB3-07 (a named-word check
    answers only its named word, with no word, outcome table or recommendation field).
  - `c600-display/tests/jumbling.rs`: CT-RPI-08's jumbling rows (produced and validated; unsupported and missing data
    explicit), CT-RPI-12's off-lattice part on a real jumbling state (with C2's stale half, CT-RPI-12 runs whole) and
    CT-RM-07's jumbling half on M5's model (rows 1 to 8 and 15).
- Not built: Jumble on in the command layer and oracle equality against M5's fixtures (KN5's one implementation, LP2);
  M8's items (nu, the checkpoints, the sticker test, a whole word's Net and Strict protection, the repair list, covers,
  release order, the correction catalogue, and the rows J01 and J06 to J12); the library entries, which need the IR
  draft's collection types; CT-LB1-02's key half (LG2.18); BM5's review records; a slot action of K⁺ and an engine-named
  rotation plane in `c600-jumble`; J10's exposed-face geometry; a comparison of the posed arrays with the J1 fixture's
  later stages; the pose levels of CT-LB4-03 and CT-EV3-06's off-lattice relations, which build on C6; LP3's own test of
  LB3.4's crossing for CT-LB3-02 (M5's `crossing.rs` runs its kernel facts as CT-KN2-03); CT-LB1-03's kernel half.
- Drafts, which freeze with the IR and the command table, are in each crate's documentation under "Drafts".
- Evidence kind: source and synthetic. No Windows, DirectX, input or timing claim. The checks ran on Windows only; the
  Linux leg was not run.
