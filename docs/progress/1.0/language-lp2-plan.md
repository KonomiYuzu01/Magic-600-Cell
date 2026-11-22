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
| Jumbling half of the kernel API | May start; EN3's engine half is approved | Nothing: after a fresh Claude read of the jumbling interfaces and its fixes, the owner approved their engine half, JI.1 to JI.16, JI.52 and JI.53, as M5's input ([owner, 16 November 2026](../../wiki/decisions/owner-decisions-2026-11-16-en3-engine-half.md); PHS.53, JI.54); the store, command and view halves stay drafts |
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
| M5 | Jumbling half: public J1 API (JI.52), differential comparisons, W-J fixtures and H6030, with CT-JI-03 (section 11) | EN3's engine half (approved 16 November 2026) |
| M6 | Session store below the language; budgets and cancellation end to end (section 10) | M2, M4 |
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

LP2 stops and reports to the owner when every remaining milestone waits for an owner answer or another line, or
when an implementation would change a frozen file (PHS.52). A failing
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

## 10. M6: the session store below the language

The owner asked on 16 November 2026 for M6 to follow M2 to M4, in a pull request of its own. M6 adds the crate
`rust/c600-session` on `c600-kernel` and `c600-run`, with the standard library only. Its drafts are listed in
`rust/README.md`; they freeze with chapter 01's open rows, the command table (LP3) and the persistence encodings, and
may change until then.

What M6 builds:

- A disk interface over a directory and over memory. The memory disk fails a chosen write, tears an unsynced append
  and loses unsynced data in a crash; the store counts its reads.
- An append-only journal. One durable action is one frame with its length, the length's complement and SHA-256,
  synced before the reply (R-06). Opening cuts a torn final frame; any other damage opens the session read-only
  (ONE.70). Mechanical events keep their literal words with pre and post digests in a parent-linked tree, and head
  changes join the action chain (PS1.22, PS1.29, PS1.43). A checkpoint's state is a content-addressed object, written
  before the frame that cites it.
- RevisionRef counters (KN0.2), snapshots by RevisionRef or of the current revision (LI.11), checkpoints, branches with
  one active line (KN4.6, ONE.73 to ONE.76), and a status that reads no storage (R-13).
- The commit path of chapter 08 section 7, from step 3: a shell stub mints single-use tokens under a per-launch secret
  (HMAC-SHA256 over `c600-digest`; KN5.14, KN5.15); receipts and outcome lookup (KN5.21 to KN5.23, LI.30); undo and
  redo with `ActionHint` (LI.9, LI.26); workspace records with M3's provenance blocks (PS1.16); branch switches and
  checkpoint saves as durable rows.
- Program jobs that run `c600-run` on opened snapshots under a declared KN7.7 budget, in the exclusive and program
  lanes (LI.22, LI.24); rows that hold no lane, such as status reads and workspace edits, run as KN7.2's `none` lane.
  `job.stop` or a commit that replaces their revision cancels them cooperatively; file and pinned jobs finish (LI.25,
  KN7.5). Results settle as LI.28 says.

Acceptance:

1. `cargo test --locked --offline` passes in `rust/`, in CI through `tests/test_language_rust.py`.
2. CT-KN4-02: a failure injected at each write of each kind of durable action, with and without a crash after it,
   leaves the session wholly before the action, or wholly after it once its frame is durable. A proved rollback
   answers `failed` with `KN4:resource_limit.disk_full`; an unproved fate answers `unknown_outcome` and keeps the
   session read-only until it is reopened.
3. CT-KN4-03: status reads no storage at 0, 50, 200 and 400 checkpoints; store reads are counted, not timed.
4. CT-KN4-04: a commit after an undo keeps the old line, exactly one line is active, a line switch is a row of kind
   `commit`, a program job compares two lines through opened snapshots, and no runtime request creates, switches or
   changes a line.
5. CT-KN4-01, its in-process part: a program job has no row that opens a session file, and `c600-run` makes no
   file-system call of its own.
6. CT-KN5-01 with PHS.19's preview-swap row, CT-KN5-04, CT-KN5-06, CT-KN5-09, CT-KN5-11, and LI.33's gate-order trace.
7. CT-KN7-01 to CT-KN7-05 and CT-KN7-07, with PHS.19's budget row: each budget case, repeated warm and after an
   eviction, stops at the same step with the same coverage and charges.
8. Persistence is a critical path: a fresh Claude subagent on a smaller model reads the change before the pull request
   merges, and its blocking findings are fixed (`AGENTS.md`, "Codex minimum").

Not in M6: the live twist; the crossing and the Jumble rows; the latest-wins and view lanes; previews and workspace
writes by programs through the session (LI.17); the runtime in a process of its own (the rest of CT-KN4-01);
historical workspace versions; the session lock; file publication; CT-KN7-06 (performance, LP3). No file under
`docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Stop condition: M6 ends when its items pass in CI and its pull request is merged. It stops earlier, and the owner is
told, if a frozen file would change or a product dependency is needed. What is not built or not run is recorded in
`rust/README.md`; non-applicability is no pass (PHS.63).

## 11. M5: the jumbling half of the kernel API

Crate: `rust/c600-jumble`, which depends on `c600-kernel` (its archive and NPY readers) and on the approved
`num-bigint` (section 7, question 1). It reads `assets/model.npz` and builds the exact jumbling geometry itself: the
exact poles, K⁺ from the stored rotation permutations, one exact double description per K⁺-orbit representative, and
the cap regions. Nothing is exported from J1, so the comparisons test an independent candidate behind the boundary
(PHS.18). A JSON-lines adapter binary serves the differential harness through a standard-library adapter in
`research/jumbling/differential/candidate/`. J1 gets its public API, `research/jumbling/sim/api.py` (JI.52), and the
harness bindings reach J1 only through it. EN3's engine half (JI.1 to JI.16, JI.52, JI.53) is M5's input; what that
draft leaves to M5 (JI.54) is decided below and recorded in the pull request.

Selections (the owner may overrule them):

1. **Exact numbers.** Q(√5) numbers are reduced integer triples on `num-bigint`, and the crate has no float type. The
   stored binary64 normals are read as exact dyadic rationals, and each coordinate is matched to the unique
   (p + q√5)/2 with |q| ≤ 12 within 10⁻⁹ by exact comparisons: J1's rule, decided exactly.
2. **Records.** The candidate reads the record families the comparisons use: `menu`, `matrix` and `cayley`. Another
   family is an adapter error, not a refusal.
3. **Free mode.** An internal entry, a start without a menu, admits any exact rotation that fixes the grip's pole. It
   serves the tests (CT-KN2-02) and is no product command.
4. **Net protection** at pose level reports its level (poses), its mode (net), the verdict and the sorted IDs of the
   protected pieces whose pose would change, and no first refused prefix, since Net judges the net map; M4's
   label-level Net does the same.
5. **Batches.** `prefix_observations` and `seam_facts` take a positive maximum chunk size (letters or cells) and a
   budget (letters or cells); cancellation is checked between chunks; a cut-off or cancelled batch has half-open
   coverage and no total or final claim (JI.14). JI.16 runs with chunk size 1.
6. **Budgets.** A classification takes an optional budget of exact evaluations. Exhausting it gives `uncertain` and
   changes nothing, and the same budget gives the same outcome on every run.

Acceptance:

1. `cargo test --locked --offline` passes in `rust/`, in CI through `tests/test_language_rust.py`.
2. JI.52: `research/jumbling/sim/api.py` is the only change under `research/jumbling/` outside `differential/`;
   `accept.py` and `fixtures/wj.py check` give their recorded results; `tests/test_jumbling_*.py` pass; a source scan
   of the bindings (adapters, `candidate/`, `references/`, `leads/census.py`, `leads/x5_release.py`) finds no private
   member of J1 or of the theory code.
3. CT-KN2-01, jumbling half (with M4's half the CT runs whole): `tests/test_jumbling_rust_candidate.py`, listed in
   `tools/ci/run_headless.py`, compares J1 with the candidate through `harness.compare` step by step (status, moved
   count, moved set, pose digest and lattice digest, refusals first) on the first 60 records of each W-J fixture (S4,
   I_a, I_b) with surveys at steps 0 and 30, the first 600 letters of H6030, and refusals: a letter outside the menu,
   a blocked twist and a malformed record. Straddler sets at named caps are equal, and every candidate certificate
   passes J1's independent check. Under `JUMBLING_FULL_REPLAY=1` every record of the three fixtures and all 3,156
   letters of H6030 run. A planted difference fails at its step.
4. CT-KN1-02: a word recorded under the reversed I_a order is refused: same menu identity, other order digest.
5. CT-KN1-05, poses half (with M4's labels half the CT runs whole): the candidate's pose digest equals J1's at every
   compared step of every fixture.
6. CT-KN2-02, in part: heights and domain IDs equal `trees.py` and the library's histograms on H6030's compared
   states and on the in-repository state of ENG-0J item 8 (u1). The free-mode case: the Cayley rotation ω = (2,0,0) at
   cap 0 reads heights (0,0), lies outside K⁺, and its domain ID, from the exact coset key (JI.53), differs from the
   identity's. Item 8's other states (b20, b06-14, b14, b03) need recorded draft inputs outside the repository and run
   in M8.
7. CT-KN2-03: with centre piece 0 protected, a letter at grip 0 that turns it by an element of A4 is refused at pose
   level inside Jumble with the report of selection 4, and the same retained word passes at label level after the
   crossing at solved: under M4's kernel no slot of the piece moves.
8. CT-KN2-04, jumbling half (with M4's half the CT runs whole): a blocked, an invalid and an uncertain (budget) letter
   each leave the pose digest, the lattice digest and the pose table unchanged.
9. CT-KN2-05: a scan of the sources of `c600-kernel` and `c600-jumble` finds no `f32` or `f64`; no INV-4 site (display
   plans, the A2 filter, float requests) is in these crates.
10. CT-KN3-02: synthetic pose entries beyond 2^64 and a 6,775-digit integer round-trip exactly through the candidate's
    pose-table input and output.
11. CT-JI-03: `seam_facts(all)` counts off-grip cells (a state with off-grip mismatches gives total_N equal to the
    on-grip plus the off-grip count, and a supplied table with one on-grip and two off-grip mismatches gives 3); JI.16's
    prefix observations, units and word check agree with exact replay through J1 (two admissible rows; F, D and ΔΦ
    zero; peak left height 2; excursion 0; tame; end digest solved; one unit of the pieces item 12 moved, which lands;
    total_N 0 at both ends); cancelling after row 0 gives coverage [0,1) with no row 1 and no final claim.
12. The pull request records JI.54's items: the Net refusal report, the identity blocks of the S4 and I_b fixtures, the
    chunk sizes and budgets, the free-mode entry, and whether JI.16's vector holds as written.
13. Model and protection logic is a critical path: a fresh Claude subagent on a smaller model reads the change before
    the pull request merges, and its blocking findings are fixed (`AGENTS.md`, "Codex minimum").
14. No file under `docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Not built in M5, each with the milestone that owns it (non-applicability is no pass, PHS.63):

- CT-KN1-03 (a projection off the lattice) and CT-KN1-04 (no lift to a finer level) need the level-crossing calls (A3,
  A5 checkpoints): M8.
- CT-KN1-06 (shapes as multisets of posed regions) and CT-KN2-06 (gauge transfer) need the shape and gauge
  comparisons: M8.
- CT-KN2-07 needs streaming replay (KN2.10), built in neither half: M8.
- CT-KN3-01 (small margins) and CT-KN3-03 (the A2 filter): the candidate has no A2 filter and the state contract records
  no small-margin cases; M8 builds the adversarial set for both.
- CT-KN3-04 to CT-KN3-06 (A6 request decoding, menu drags, the shell producer): M8.
- R-11's jumbling property (CT-LG2-12, LG2.14's four hypotheses) needs the language's binding to the jumbling calls:
  M8.
- The other JI.6 and JI.7 calls (`rim_summary`, `covers`, `repair_list`, `nu`, the three checkpoints, `release_check`,
  derivation with reachability) and Strict protection: M8. `rust/README.md` lists what is built.

Stop condition: M5 ends when items 1 to 14 pass in CI and its pull request is merged. It stops earlier, and the owner is
told, under M1's conditions, or when J1 and the candidate differ and the difference is not the candidate's: a J1 fault
is reported, never fixed in J1, while the research is stopped.

## 12. M8: LP2's exit record

Written on 22 November 2026 for the owner's sign-off, under PHS.4. It is no sign-off: LP2 is not marked exited and no
progress item is ticked until the owner signs LP2's exit, on one card with LP3's C10
([owner, 22 November 2026](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md), items 6 and 8).
Evidence class throughout: source and fixture (PHS.19). Nothing here is Windows interaction, DirectX, long-session or
performance evidence.

- **Candidate.** `0260e6b1f17485a1e3689e867c73435c6cfe5aac`, main after
  [pull request 401](https://github.com/KonomiYuzu01/Magic-600-Cell/pull/401). LP2 exits on it, with no new candidate
  (item 6). Changes merged after it, pull request 404 among them, are not part of it; none of them edits a line that
  names a CT ID, so the index's marker states are the candidate's.
- **Products (PHS.18).**

  | Product | Where |
  | --- | --- |
  | Kernel API over both puzzles | `c600-kernel` (600-cell-Full, M4) and `c600-jumble` (the jumbling half with the public J1 API of JI.52, M5) |
  | 600-cell-Full candidate adapter | `c600-kernel`'s oracle adapter: O01 to O07 and N01 to N04 run through the kernel API (`tests/test_kernel_oracle.py`) |
  | Language core | `c600-syntax` (parser, M7), `c600-ir` (typed IR draft `c600-ir-draft-1`), `c600-check` (checker, M1), `c600-run` (interpreter, `lp1-exec-draft-1` meter, budgets and cooperative cancellation, M2 and M3), `c600-session` (session store and program jobs, M6) |
  | Conformance suite | Each crate's `tests/` with PHS.62 markers; `tools/language/ct_index.py` computes section 15.3's index from them and [the index notes](lp2-ct-index-notes.md) |

- **Applicable owner decisions.** The language route (LQ8, LQ9, LQ21;
  [14 November](../../wiki/decisions/owner-decisions-2026-11-14-language-route.md)); the freeze and LP1's sign-off
  ([15 November](../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)); big integers, LQ13, EN3's engine half,
  LP2's questions (the headless Linux leg on CI) and five PHS.52 readings
  ([16 November](../../wiki/decisions/owner-decisions-2026-11-16-lp2-questions.md), with the
  [big-integer](../../wiki/decisions/owner-decisions-2026-11-16-big-integers.md),
  [LQ13](../../wiki/decisions/owner-decisions-2026-11-16-lq13.md),
  [EN3](../../wiki/decisions/owner-decisions-2026-11-16-en3-engine-half.md) and
  [PHS.52](../../wiki/decisions/owner-decisions-2026-11-16-phs52.md) pages); the commit boundary's readings
  ([18 November](../../wiki/decisions/owner-decisions-2026-11-18.md)); LP3's exit items, among them no row-answer
  cache and form 2's vectors kept out of the run
  ([20 November](../../wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md)); the product budget ceilings
  ([21 November](../../wiki/decisions/owner-decisions-2026-11-21-ceilings-e3.md)); and the exit on this candidate with
  the readings below ([22 November](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md)).
- **Checks on the candidate**, all green:
  - [Windows headless checks, run 44871659856](https://github.com/KonomiYuzu01/Magic-600-Cell/actions/runs/44871659856):
    the plan, five shards and the headless job.
  - [Linux Rust tests, run 44871659853](https://github.com/KonomiYuzu01/Magic-600-Cell/actions/runs/44871659853):
    `cargo test` in `rust/` on ubuntu-24.04 and six J1 comparisons (attacks, M8 observations, corrections, candidate,
    M8 levels and ν).
  - [lp2-full, run 44871666684](https://github.com/KonomiYuzu01/Magic-600-Cell/actions/runs/44871666684), dispatched on
    the candidate, 25 jobs: the full replay (`JUMBLING_FULL_REPLAY=1`) of the candidate, the corrections and the
    command route; the jumbling layer on in the command layer with whole fixtures (release build, `--ignored`); ten
    transcript checks on each of windows-2025 and ubuntu-24.04; and the comparison of the Windows transcripts against
    the Linux ones.
- **PHS.19's acceptance items.** Each is source evidence, and each check passed in the runs above.

  | Item | Evidence on the candidate | Index |
  | --- | --- | --- |
  | Both oracles pass on the API | `tests/test_kernel_oracle.py` (O01 to O07, N01 to N04 through the kernel API); `tests/test_jumbling_rust_candidate.py` against J1, every record with the full replay in lp2-full; the jumbling layer's whole fixtures in lp2-full | CT-KN1-01 whole by union, CT-KN1-02 whole |
  | Cross-platform conformance and determinism | lp2-full's transcript comparison (windows-2025 against ubuntu-24.04); the Linux Rust job | CT-PS1-01 whole (`c600-run/tests/canonical_ps1.rs`), CT-DET-15 whole (`c600-display/tests/ct_rpi_19.rs`) |
  | Exact decision paths | `tests/test_kernel_oracle.py` and `tests/test_jumbling_rust_candidate.py`, refusals included | CT-KN2-05 whole by union |
  | Preview swap refused | `c600-session/tests/commit.rs` | CT-KN5-01 whole |
  | Repeatable budget exhaustion | `c600-session/tests/jobs.rs`; there are no cache or eviction variants, since 1.0's host keeps no row-answer cache (20 November, item 1) | CT-KN7-01 and CT-KN7-02 whole |
  | Centre-0 protection | `c600-jumble/tests/crossing.rs`, `tests/test_jumbling_rust_candidate.py` | CT-KN2-03 whole |
  | Heights and domain IDs agree | `tests/test_jumbling_rust_candidate.py`, with the long states under the full replay in lp2-full | CT-KN2-02 whole |
  | R-11's jumbling property | `c600-jumble-tool/tests/lb3.rs`'s fixtures under LG2.14's four hypotheses, with the language and kernel halves | CT-LG2-12, CT-LG2-13 and CT-LG2-14 whole by union |
  | Gauge transfer | `tests/test_jumbling_rust_m8_levels.py`; a pass adds no proof (KN1.14) | CT-KN2-06 whole |

- **The index.** On the candidate, `ct_index.py` gave 248 IDs: 126 whole (32 of them by union), 121 part, none named
  only, none missing, 1 waiting (CT-LI-14). Section 1 asks that all 248 run whole; 122 do not, and the owner read each
  of them (item 8). With the readings in the index notes (this record's pull request changes notes and the tool's
  note kinds only, no test or marker), it gives 126 whole (32 by union), 7 not applicable, 8 waiting and 107 indexed
  elsewhere, none part. None of the 122 counts as a pass (PHS.63). The notes name each ID with its readings; 16 IDs
  fall under more than one.
- **The readings and their decisions** (owner, 22 November 2026, item 8; every reading takes its recommended option):

  | Reading | What LP2's products cannot run | LP2 IDs | Decision | Index state |
  | --- | --- | --- | --- | --- |
  | 1 | A second admitted route (RT1 is the only one, LQ9) | 11 | Not applicable while RT1 is the only admitted route; reopens if another is admitted | Not applicable where no other reading applies (CT-LG4-01, CT-DET-10, CT-DET-26, CT-DET-30) |
  | 2 | ONE.32's fields in the tool-authoring and research profiles (BND.60) | 1 | Not applicable while BND.60 stands | Not applicable (CT-BND-11) |
  | 3 | Mixed dependency editions | 1 | Not applicable until a second edition is supported | Not applicable (CT-LG1-01) |
  | 4 | A budgeted research search near the limit | 1 | Not applicable until the research profile budgets it | Not applicable (CT-KN3-04) |
  | 5 | LQ2's joint setting and BND.37's class review | 8 | Waiting; the tests run when those land | Waiting |
  | 6 | LP4's editor services | 5 | The missing halves are indexed in LP4 | Indexed elsewhere |
  | 7 | LP5's checkers | 7 | The missing halves are indexed in LP5 | Indexed elsewhere |
  | 8 | Form 2 (LG2.21), which has no executor | 2 | Amended under PHS.52 (chapter 12, below section 15.3's table): form 2 stays out of LP2 until it is built | Indexed elsewhere |
  | 9 | LP3's host, shell, display and renderer | 22 | Indexed with the work that builds each piece | Indexed elsewhere |
  | 10 | LP3's host serving frozen H-08 rows that no crate serves | 8 | Indexed with LP3's host | Indexed elsewhere |
  | 11 | The command-layer tests | - | Closed by pull request 401 | - |
  | 12 | M6's store and run record beyond what M6 built | 24 | Indexed with the store work that builds each row | Indexed elsewhere |
  | 13 | The importer | 4 | Indexed with the importer once the owner places it | Indexed elsewhere |
  | 14 | IR constructs the draft lacks | 19 | Indexed with the IR's freeze | Indexed elsewhere |
  | 15 | Chapter 03 operations no crate builds | 3 | Indexed with chapter 03's library work | Indexed elsewhere |
  | 16 | Kernel and jumbling calls beyond M4 and M5 | 12 | Indexed with the work that adds each call | Indexed elsewhere |
  | 17 | Runs only the owner's machine or another platform can give | 8 | Owner-machine halves join LP3's owner-machine batch; other platforms come after 1.0 | Indexed elsewhere |
  | 18 | Undo of a language-defined workspace row (LG3.5, LB7.2) | 2 | Option 1: LG3.5 and LB7.2 stay; the row's completion gets an undo entry like `filter.apply`'s, which LP3's command layer adds after the exit; the undo halves of CT-LG3-03 and CT-LB7-01 move to that fix (PHS.52, item 6) | Indexed elsewhere |

- **What stays open.**
  - PHS.21's items: the three-tool loop, native renderer integration, authoring services, full research corpus
    checking and the path B importer.
  - The 107 indexed halves, each with the work its note names, reading 18's undo fix among them.
  - The 8 waiting IDs, until LQ2's joint setting and BND.37's class review land.
  - The 7 not-applicable IDs, which reopen if their recorded decision changes.
  - Form 2 (LG2.21), which is not built.
  - The markers of CT-DET-05, CT-DET-06 and CT-DET-29 still say the headless Linux leg has no owner. Since the owner's
    16 November answer, the Linux Rust job runs them on ubuntu-24.04 (run 44871659853). The candidate keeps the text,
    and the index takes those IDs from their notes, not from that remark.
  - LQ2's option set, the design-system ADR, and the syntax and IR freeze (PHS.49).
  - The owner's sign-off of LP2's exit, on one card with C10. The integrator ticks M8's acceptance item after it.
