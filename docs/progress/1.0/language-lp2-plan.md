# Language LP2 plan: the headless core

Status: plan of 15 November 2026, written when LP2 started after the LP1 sign-off
([owner decisions of 15 November 2026](../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). It records the
integrator's selections, and the owner may overrule any of them. It is no ADR and freezes nothing.

Inputs: chapter 12 sections 4 and 15.3 (PHS.18 to PHS.21, PHS.55, PHS.62 to PHS.66); chapter 02 (LG2.5,
LG2.19 to LG2.22, LG3, LG4); chapter 07 (BND.39, BND.42 to BND.44); chapter 09 (DET.54 to DET.64); chapter 11
(RTE.18, RTE.70 and the phase table); the frozen [contract vectors](../../architecture/1.0/language/contract-vectors/README.md);
the owner decisions of 14 and 15 November 2026.

Evidence kind for everything this plan produces: source and fixture. Nothing here is Windows interaction, DirectX,
long-session or performance evidence.

## 1. What LP2 delivers

PHS.18's four products: the kernel API over both puzzles, the 600-cell-Full candidate adapter, the language core
(parser, checker, typed IR and execution contract, interpreter, budgets and cooperative cancellation) and the
conformance suite for the available products. LP2 exits only when PHS.19's nine acceptance items have evidence on one
candidate and the 248 tests that section 15.3 indexes run whole (PHS.62, PHS.63, PHS.66). The three-tool loop, renderer
integration, authoring services, the research corpus and the importer stay with later phases (PHS.21).

## 2. What may start now and what waits

| Work | State | Waits for |
| --- | --- | --- |
| Typed IR draft and checker | May start; LQ21 is answered | Nothing |
| Interpreter and the `lp1-exec-draft-1` meter | May start; the big-integer question is answered | Nothing: the owner chose option A, `num-bigint` 0.5.1 with `num-integer` 0.1.47 and `num-traits` 0.2.19 ([owner, 16 November 2026](../../wiki/decisions/owner-decisions-2026-11-16-big-integers.md); RTE.18, RTE.70) |
| Parser | May start; LQ13 is answered | Nothing: the owner chose LX-8 candidate A (braces) from a one-page comparison of the three candidates, with no human test ([owner, 16 November 2026](../../wiki/decisions/owner-decisions-2026-11-16-lq13.md)); keyword and punctuation details are settled in M7, and CT-RTE-18 stays recorded as not run |
| Kernel API and candidate adapter, 600-cell-Full half | May start in Rust (LQ8) | Nothing beyond the frozen chapters |
| Jumbling half of the kernel API | Waits | EN3's review and approval of the jumbling interfaces (PHS.53) |
| Session store below the language | May start | Nothing beyond chapter 05's frozen core |

## 3. Integrator selections (the owner may overrule them)

1. **Exactness form.** LP2 builds form 1 of LG2.5, `ApproxBool` and `display_if`, as HANDOFF H6 and LG2.5 recommend.
   Form 2 (`lp1-exec-region-draft-1`) stays on record and is not built. The checker and the IR advertise the
   exactness and execution contract `lp1-exec-draft-1` (LG2.21, DET.54) and refuse a program pinned to another one
   with `LG4:input.ir_version_unsupported`. CT-LG2-05's form-2 half is therefore not applicable while form 2 is not
   built; this record says so, and non-applicability is no pass (PHS.63).
2. **Where the code lives.** One Cargo workspace in `rust/`, one crate per layer, named `c600-*`. The IR crate holds
   data only; the checker and the later interpreter depend on it and not on each other. No external crate enters
   without the owner's approval (RTE.70); each approved one is listed with its licence in the workspace README.
3. **Runner, directories and records (PHS.65).** The runner is `cargo test --locked --offline` in `rust/`, which CI
   runs through `tests/test_language_rust.py`, listed in `tools/ci/run_headless.py`. Tests live in each crate's
   `tests/` folder. A test that implements part of a conformance test names the ID and the part; the ID counts as run
   only when its whole definition runs (PHS.62, PHS.63). The evidence record is the CI run of the merged commit,
   named in the pull request. No new commit guard.
4. **IR status.** The IR is a draft, `c600-ir-draft-1`, and changes freely until the IR freeze (PHS.49). Until the
   parser exists, tests build IR by hand; a hand lowering is no evidence for the parser.

## 4. Milestones

One pull request per milestone; each names the checks that ran. At the owner's request (16 November 2026), M2 to M4
share one pull request, which names each milestone's checks (section 9).

| Milestone | Content | Depends on |
| --- | --- | --- |
| M0, done | CI runs the eight language checkers ([pull request 148](https://github.com/KonomiYuzu01/Magic-600-Cell/pull/148)) | - |
| M1 | IR draft and checker: exactness, effects and capabilities, views, capability separation, grants (section 5) | - |
| M2 | Interpreter and `lp1-exec-draft-1` reference meter: the 40 frozen traces and the meter boundary vectors | Big-integer answer (answered: option A) |
| M3 | Run-time boundary: stored provenance, the fixed acknowledgement of BND.43, the saved-word half of CT-LG3-07 | M2 |
| M4 | Kernel API and 600-cell-Full candidate adapter: O01 to O07 through `tools/oracle/` | Rust engine port |
| M5 | Jumbling half: public J1 API, differential comparisons, W-J fixtures and H6030 | EN3's approval |
| M6 | Session store below the language; budgets and cancellation end to end | M2, M4 |
| M7 | Parser and its lowering into the IR (section 8) | LQ13 (answered: candidate A) |
| M8 | PHS.19's acceptance on Windows and headless Linux; section 15.3 runnable | M1 to M7 |

## 5. M1: the IR draft and the checker

Crates: `rust/c600-ir` (the IR draft) and `rust/c600-check` (the checker). The checker implements:

- LG2.19, LG2.20 and LG2.22: the display qualifier; ordinary control and exact operands take exact inputs only;
  `display_if` takes an `ApproxBool` and pure display branches; function, closure, aggregate, storage and export
  boundaries keep the qualifier; display accounting never reaches exact code; `exactness_violation` takes precedence
  over `exact_required`.
- LG3.1 and LG3.2: effects and capability requirements are inferred through direct, recursive, higher-order, generic
  and imported calls; an annotation cannot erase an inferred effect; an unresolved callable is not pure.
- LG3.4: the IR has no commit operation and no commit effect.
- LG3.5 and LG3.6: a view has no workspace or preview effect, also through callees.
- LG3.13, BND.39 and BND.42: in the solving profile a program that prepares previews reads no state and no preview
  outcome; helpers and other modules do not launder this; no declassifier is enabled until LQ2's joint setting.
- LG3.8 and LG3.9: file, clock and randomness need the module's declaration and the run's grant; network, process and
  session-file access are never granted.

Acceptance:

1. `cargo test --locked --offline` passes in `rust/`, in CI through `tests/test_language_rust.py`.
2. The six cases of the frozen boundary vector `direct-and-helper-condition` and the three of
   `display-accounting-no-exact-continuation`, lowered to the IR by hand, give exactly the frozen codes with the
   vector's value type and its display-control and qualifier-erasure fields. The test reads `vectors.json`, so a change
   to the vector fails it. The accounting cases come from form-2 traces; the rule they check is LG2.20's, which binds
   both forms, and M1 lowers them to form 1.
3. The admissible counterparts pass: `display_if` choosing two colours gives a display colour, and storing or exporting
   it with its qualifier is admitted (LG2.22).
4. Effect, view, capability-separation and grant tests cover the M1 parts of CT-LG2-04, CT-LG2-05, CT-LG3-01,
   CT-LG3-02, CT-LG3-03, CT-LG3-04 and CT-LG3-07, each naming its part.
5. The checker's report lists diagnostics in IR order and is the same on every run (LG4.3).
6. No file under `docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Stop condition: M1 ends when acceptance items 1 to 6 pass in CI and its pull request is merged. It stops earlier, and
the owner is told, if a rule cannot be implemented without changing a frozen file or choosing an open contract
(LG2.18's word keys, LG4.8's production tariffs, LQ2's policy options, LQ13's syntax), or if it needs a product
dependency.

## 6. Stop condition for LP2

LP2 stops and reports to the owner when every remaining milestone waits for an owner answer or another line (EN3's
approval, for example), or when an implementation would change a frozen file (PHS.52). A failing
acceptance item blocks LP2's exit (PHS.63); nothing is cut.

## 7. Questions put to the owner on 15 November 2026

1. The big-integer dependency for M2: `num-bigint` 0.5.1 with `num-integer` 0.1.47 and `num-traits` 0.2.19 (each
   MIT OR Apache-2.0, as read from the verified archives in LX-5's crate run; exact on LX-5's rows and about 3.4 times
   faster in total than the experiment's own limbs), or own limbs with no dependency. Answered on 16 November 2026:
   option A ([owner decision](../../wiki/decisions/owner-decisions-2026-11-16-big-integers.md)).
2. LX-8's human-test protocol, which LQ13 and the parser waited for. Moot since 16 November 2026: the owner chose LQ13
   directly from a one-page comparison, candidate A, so the parser waits no longer.

Questions to the owner are batched (owner, 16 November 2026): at most one question card per milestone, each with a
recommended option. The big-integer card is M2's card; other M2 points follow recorded decisions or conventional
defaults.

## 8. M7: the parser and its lowering into the IR

Crate: `rust/c600-syntax`, which depends on `c600-ir` only. It checks the source bytes, parses LX-8 candidate A with
recovery, formats parsed programs and lowers them into the IR draft with a source map. The syntax details settled here
are drafts: they freeze with the syntax and the IR (PHS.49), and `rust/README.md` lists them.

Selections (the owner may overrule them):

1. **Edition.** The header is `edition "6cs-draft-1";`, the only supported edition. Like `c600-ir-draft-1`, the name
   says that its rules may still change before the syntax freeze (LG1.1, LG1.2).
2. **Grammar.** Candidate A's grammar stands: declaration keywords, braces, `let`, an explicit `return`, `match` with
   arms ending in `;`, column-1 declarations and no trailing commas. The IR draft needs a few additions, each in A's
   style: `export` before a declaration, generic parameters `fn f<T, E>(...)`, closures `fn(x: T) -> R { ... }`,
   records `record { a = e }` and field access `e.a`, literal arms and a `_` arm in `match`, type arguments in calls
   (`read_snapshot<Colour>("q")`), the types `Display<T>`, `record { a: T }` and `fn(T) -> R effects [...]`, and the
   literals `true`, `false` and `()`. The conditionals are the reserved calls `if(c, a, b)` and `display_if(c, a, b)`,
   LG2.20's notation; the IR's other operations are reserved names called like functions. `charge` and the command
   metadata become optional clauses.
3. **Exact literals** stay exact decimal text in the IR: unbounded integers, rationals with a nonzero denominator and
   the sign on the numerator, and Q(√5) pairs; an approximation keeps its decimal text and its precision (24, 53 or
   113 bits). No host float and no big-integer crate is involved. Reducing a rational to lowest terms comes with M2's
   big integers.
4. **What parses but does not lower yet.** Declared charges (LG4.8's tariffs), command metadata (the command table,
   LP3), verdict matches, menu words and LX-8's domain types (the IR's domain families, M4 and M5). Lowering refuses
   each with a message that names what it waits for.
5. **Comments** are accepted and not retained, as in candidate A, so the formatter drops them (LG1.4 allows it;
   retention comes with the authoring services, PHS.21).
6. **Source map** (LG4.12). Lowering returns, for each function, the source range of every IR path that the checker
   names in a diagnostic, with the module's source digest (SHA-256 of the pinned bytes, written in the crate).

Acceptance:

1. `cargo test --locked --offline` passes in `rust/`, in CI through `tests/test_language_rust.py`.
2. CT-LG1-01, first half: a file without the header gives `LG1:syntax.edition_missing` with an insertion range, and
   another edition gives `LG1:syntax.edition_unsupported`; nothing after the header is parsed or retried under another
   edition. The second half, mixed dependency editions, is not applicable while one edition exists, which is no pass.
3. CT-LG1-02: a `.6cs` fixture yields its pinned SHA-256 from the checkout (`* -text`); the same bytes with CRLF line
   ends or a BOM, and CT-ERR-05's inputs `61 FF 62` and `61 F0 9F 98`, give `LG1:syntax.invalid_encoding` with no
   span, the raw input digest and the offending byte interval ([1,2) and [1,4) for the two CT-ERR-05 inputs).
4. CT-LG1-03: formatting is a function of the parsed program; formatting again gives the same bytes; the formatted
   text parses to the same program and lowers to the same IR; one expected output is pinned in the test, so Linux
   and Windows compare against the same bytes.
5. CT-LG1-04: candidate A's fourteen text recovery drafts from LX-8, with the product header, give the codes and
   recovered declarations that LX-8 records; a prefix shifts every location and keeps every code; two independent
   errors both recover; a program with a recovery hole does not lower.
6. CT-LG1-05: an integer literal of more than 100 digits, rationals and Q(√5) pairs lower to exact text; `~53"0.125"`
   lowers to a distinct approximate literal; `1/0`, `1.5` and `~53"NaN"` give `LG1:syntax.invalid_literal`.
7. The nine frozen exactness cases of M1's acceptance item 2, written in source, give exactly the frozen code, value
   type and fields through the checker, and each diagnostic maps to a source range of its function.
8. Candidate A's 17 LX-8 fixtures, with the product header, parse and format idempotently, so the product grammar
   accepts every program LX-8 wrote in candidate A.
9. No file under `docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Stop condition: M7 ends when items 1 to 9 pass in CI and its pull request is merged. It stops earlier, and the owner is
told, if a rule cannot be implemented without changing a frozen file or choosing an open contract (LG4.8's tariffs,
the command table, LQ2's policy options), or if it needs a product dependency.

## 9. M2 to M4: the interpreter, the run-time boundary and the 600-cell-Full kernel

The owner asked on 16 November 2026 for M2 to M4 to be built in one go and delivered in one pull request. The drafts
each milestone settles are listed in `rust/README.md`; they freeze with the IR, the command table or chapter 01's open
rows, and may change until then.

M2, the interpreter and the `lp1-exec-draft-1` reference meter (crate `rust/c600-run`). Acceptance:

1. `cargo test --locked --offline` passes in `rust/`, in CI through `tests/test_language_rust.py`.
2. The 40 frozen contract-vector traces replay through the reference meter with every field equal; the meter boundary
   vectors derived from them stop where the frozen rows predict; a negative control fails.
3. The interpreter charges one row per operation (DET.55), observes interruption before each operation (DET.56),
   publishes DET.59's canonical bytes, draws from `sha256-counter-draft-1` (DET.61), charges only the selected
   `display_if` branch (DET.64), and stops with the call path, the operation and the source range.
4. Exact arithmetic is exact on `num-bigint`, and `Approx<p>` rounds to nearest with ties to even, against golden
   values computed independently.

M3, the run-time boundary (`c600-run`'s `provenance` and `boundary` modules). Acceptance:

1. FP.18's four example blocks digest as frozen, and the capability-form blocks are rebuilt byte for byte; a malformed
   block, a block in another form, a pin the policy does not hold and an unresolved pick give
   `PS1:capability.provenance_incompatible` (FP.17, FP.38).
2. A stored word carries the block the host built, through the workspace and through files, whatever the program
   writes, and a later run cites it (BND.22, BND.24, BND.32).
3. The saved-word half of CT-LG3-07: a word that an own, forked or imported tool saved, through the workspace or a
   file, is refused at preview admission with `LG3:capability.not_granted`, and so are research output (BND.61), a
   nested tool block and a missing part (BND.85); a typed word and an import pass; `LG3:capability.flow_refused` never
   appears (CT-BND-24).
4. BND.43: a solving run gets the same output and charge rows whether its preview was built, refused at admission or
   refused by the engine; in tool authoring and research the engine builds no preview (BND.60).

M4, the kernel API and the 600-cell-Full candidate adapter (crate `rust/c600-kernel`). Acceptance:

1. O01 to O07 run through the kernel API, and `tools/oracle/compare.py` reports each equal with no omitted fields
   (`tests/test_kernel_oracle.py`, listed in `tools/ci/run_headless.py`); a tampered trace fails at its step, and a case
   with another model identity is refused without a trace.
2. The star cache agrees with per-primitive replay (KN2.9).
3. The model is read from `assets/` with the model ID tied to the file list and every listed digest checked, and the
   crate takes no external dependency.
4. Model and protection logic is a critical path: a fresh Claude subagent on a smaller model reads the change before
   the pull request merges, and its blocking findings are fixed (`AGENTS.md`, "Codex minimum").

For all three, no file under `docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Stop condition: each milestone ends when its items pass in CI and the shared pull request is merged. Each stops
earlier, and the owner is told, under M1's conditions. What is not built or not run is recorded in `rust/README.md`;
non-applicability is no pass (PHS.63).
