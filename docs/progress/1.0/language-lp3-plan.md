# Language LP3 plan: the first closed loop

Status: plan of 16 November 2026, written when LP3's offline work started beside LP2's remaining milestones. It records
the integrator's selections, and the owner may overrule any of them. It is no ADR and freezes nothing: the command
table stays a draft until it freezes at LP3 (PHS.53), and the IR until its own freeze (PHS.49).

Inputs: chapter 12 sections 5 and 15.4 (PHS.22 to PHS.25, PHS.61 to PHS.67); chapter 01 KN5; chapter 08 section 7;
the [layer interfaces](layer-interfaces.md) (LI.5, LI.6, LI.9, LI.26, LI.30); H-08's stored
[command table](command-table.json); ADR-L02; chapter 03 (LB1 to LB6); chapter 10 and the renderer feature matrix; the
owner decisions of 14, 15 and 16 November 2026; the [LP2 plan](language-lp2-plan.md).

Evidence kind for everything this plan produces without the owner's machine: source and fixture, and synthetic where a
test's class says so. Nothing here is Windows interaction, DirectX, input, long-session or performance evidence; the
timings the checks print are informational. The windows and performance tests run on the owner's machine under PHS.64.

## 1. What LP3 delivers

PHS.22: three built-in tools written in the language above the kernel, one for 600-cell-Full, one for jumbling and one
for representations, each demonstrating the complete loop. Its four products:

| Product | Without the owner's machine | Waits for |
| --- | --- | --- |
| Three-tool vertical path: source, check, run, view nodes, engine preview, explicit user commit, durable receipt and undo | 600-cell-Full: preview, commit and undo through the command layer with oracle equality (C1), the source route (C5), view nodes (C2, C5). Representations: the headless half (C6) | Durable receipts and undo through the command layer: KN5's one implementation, which LP2 writes in `c600-command` (C7). The jumbling tool: M5 (C8) |
| Matching command and key routes, kernel-row keys on ONE.48's direct path | The command and key routes with byte-identical traces and receipts (C1); the source route (C5) | The actual key route on Windows (C9) |
| Active program views under ONE.11, ONE.12 and ONE.64 | Revision-bound output, latest-wins recomputation and stale-state data, headless (C2) | On screen: the renderer freeze (C9) |
| Further views and user stages | `view.open`, `view.close` and RPI.82's budget, headless (C2) | User vertex and fragment stages on the selected renderer, advertised only after RPI.79's run (C9) |

PHS.23's six acceptance items:

| Acceptance item | Without the owner's machine | Waits for |
| --- | --- | --- |
| Same result through all routes | Command and key routes (C1); source route (C5) | The representations tool's rows: KN5's one implementation (LP2) and an IR list type (C6, section 7); the actual key route (C9); the jumbling tool (C8) |
| Oracle equality at every committed step | 600-cell-Full: O01 to O07, 3,201 steps per route, with undo and redo (C1) | Jumbling: M5's oracle (C8) |
| Presented-preview binding in the selected renderer | The token binds the presented preview, headless (C1) | The renderer freeze and Windows (C9) |
| Charter E2 with the tools' views active | Nothing: a timing here would be no evidence | The owner-machine runs (C9) |
| Stage and view contract on the selected renderer | The headless display contract (C2) | The renderer freeze, Windows and the gate run (C9) |
| Decisive LX-1 to LX-4 checks | The headless parts: the execution-contract mapping and LX-2's faults (C3), module closure (C4) | The owner-machine runs (C9). A failing LX-1, LX-2 or LX-4 run reopens LQ9; a failing LX-3 check goes to the owner |

LP3 exits only when every acceptance item has evidence on one frozen candidate and the 82 tests of section 15.4 run
whole: 50 source, 13 synthetic, 12 windows and 7 performance (PHS.62, PHS.63, PHS.66). LP3 depends on LP2, frozen H-08
and the renderer freeze for its on-screen half, and headless integration does not pass the on-screen acceptance early
(PHS.24).

## 2. What may start now and what waits

| Work | State | Waits for |
| --- | --- | --- |
| Command layer, shell tokens and key routes; the oracle loop for 600-cell-Full | Built (C1) | Nothing; the table freezes at LP3 |
| Headless display contract; views and their budgets | Built (C2) | Nothing |
| The runtime in its own process | Built (C3) | Nothing: route RT1 (LQ9) |
| Module closure | Built (C4) | Nothing |
| Source route and the 600-cell-Full tool | Built in part (C5, section 7) | The parts section 7 names for LP2; the jumbling halves for M5 (C8) |
| Representations tool, headless half | Built in part (C6, section 7) | Its six rows: command-layer work in KN5's one implementation (LP2), and a list type in the IR draft for `set.save` |
| Durable receipts and undo, CT-RTE-08's persisted words, work sheets | Built in part on M6's store (C7, section 8) | Receipts and undo through the command layer: KN5's one implementation, which LP2 writes in `c600-command` (PHS.18) |
| The jumbling tool and every jumbling half | Waits | M5 on main |
| Tests that read kernel rows LP2 has not built: Strict protection, streaming words (KN2.10), `positions` (KN2.4), KN3's exact rotations and polytope operations | Waits | LP2's kernel: LP2 deliverables, not built yet (the table below) |
| The on-screen half | Waits | The renderer freeze |
| Charter E2, the decisive LX-1, LX-2 and LX-4 runs, LX-3's platform coverage, the renderer gate | Waits | The owner's machine, all run together (owner, 16 November 2026) |
| CT-LB1-06's word keys | Waits | An owner answer on LG2.18's word keys (section 10) |
| Library entries as language code (LB1.1): collections, exact linear algebra, word construction, word and trace keys, setup search | Waits | Collection types and word operations in the IR draft (LP2; the IR freezes at PHS.49): LP2 deliverables, not built yet (the table below) |

LP2 deliverables that LP3's tests wait for, not built yet. The LP2 line confirmed on 16 November 2026 that each is
LP2 work: section 15.3 lists its tests, and PHS.18's product row "Kernel API over both puzzles" and the LP2 plan's
M8 row ("section 15.3 runnable") cover the kernel rows. LP3 builds nothing in LP2's crates.

| LP2 deliverable, not built yet | Rule | LP2 tests | LP3 tests that wait |
| --- | --- | --- | --- |
| Strict protection on 600-cell-Full | KN2.4, KN2.7 | None names Strict on 600-cell-Full; CT-KN2-03 and CT-LG2-12 use the `protection` row | CT-LB2-02 |
| Streaming words | KN2.10 | CT-KN2-07, CT-LG2-14 | CT-LB2-03 |
| The `positions` row | KN2.4 | None names it | CT-LB2-04 |
| Exact rotations and `twist.apply` (amendment A6) | KN3.4, KN3.7 to KN3.9 | CT-KN3-04 to CT-KN3-06, CT-LG2-16 | CT-EV4-04's twist leg, CT-BND-19's live-twist part |
| Exact polytope operations | KN3.3 | CT-KN3-01, CT-KN3-03 | CT-LB4-01 |
| The F2 and F5 number kinds | KN3.2 | None names them; the `nu` and `correction_set_check` rows use them (M5) | CT-LB1-01 |
| Word operations in the IR draft: concatenation, inverse, conjugate, commutator | LG2.13 to LG2.16 | CT-LG2-11 to CT-LG2-14 | CT-LB1-03, CT-LB1-06, CT-LB2-01, CT-BND-18 |
| Collection types in the IR draft: lists, maps, sets | LG2.20, LG4.3 | None names them; CT-LG4-01 checks host collection layouts | CT-LB1-01, CT-LB1-03, CT-LB1-06, CT-LB2-01, CT-BND-18 |
| The IR features CT-RTE-04's other 32 traces need, and two meter differences in `c600-run` (`nested-calls`'s frame price; an `emit` row's single charge) | DET.56, DET.57, DET.65 | DET.65's trace vectors in `contract-vectors/vectors.json` | CT-RTE-04 |

Found later, on 16 November 2026. KN5's one implementation, which LP2 writes in `c600-command` on branch
`claude/lp2-kn5-one`, also carries the presented-frame input to `present` that CT-ONE-29's shell-side refusal needs and
the requirement from LP3's KN5.21 lead, both accepted by the LP2 integrator. Sent to the LP2 integrator the same day and
not yet confirmed: `c600-check` rules that decide CT-LB3-01's checker half (typed checkpoint and sticker-test results,
LB3.2) and CT-LB3-07's recommendation rule (LB3.5); an author or column parameter on `c600_check::admit`, with a field
in `c600-runtime`'s job options, so the runtime's child applies the built-in column itself; and a list of words in the
IR, or KN2.10's streaming, for chunked recipes on the source route.

Found with C6, the same day. The representations tool's six rows need command-layer work that KN5's one implementation
now owns: a dispatch from a stored row to a built-in tool's entry, the runtime's view and workspace sinks as a job's
result, argument forms for the rows' reference types, and a workspace revision that moves (`c600_repr::rows`);
`set.save` also needs a list type in the IR draft. CT-EV3-07's explicit workspace command joins the durable action order
on M6's store (C7), but through the command layer it needs KN5's shared `session.undo` order (KN5.20, EV4.15), so the
test moves from "whole now" to "part now". Sent to the LP2 integrator the same day.

## 3. Integrator selections (the owner may overrule them)

1. **Crates.** One crate per layer in `rust/`, named `c600-*`, with no external crate: `c600-command` (the command
   layer and the shell; it depends on `c600-kernel` and `c600-digest` only, so it has no path to the interpreter,
   ONE.48), `c600-display` (the headless display contract), `c600-view` (views and their budgets), `c600-runtime` (the
   runtime's own process, RT1), `c600-module` (module closure) and, from C5, `c600-tools` (the built-in tools' sources
   and their host bridge). A change to an LP2 crate's API is agreed with the LP2 line first and kept small.
2. **Command-table drafts.** The layer reads H-08's stored table as it is. Its draft choices are listed in
   `c600-command`'s crate documentation and `rust/README.md`, and freeze with the table at LP3 (PHS.53). Codes the
   layer raises that chapter 13 does not register yet are drafts; registering them changes a frozen chapter, so it goes
   through PHS.52 at C10.
3. **Session store stand-in.** Step 9's durable transaction sits behind a `Store` trait. `MemoryStore` stands in until
   KN5's one implementation (PHS.18), which LP2 writes in `c600-command` with `c600-session` implementing the trait,
   replaces it, so "durable" means only "kept across the layer's restart". Nothing on `MemoryStore` is persistence
   evidence; C7's tests on M6's store are.
4. **Routes.** The command route invokes rows with absolute arguments. The key route presses the draft keymap's chords
   (Ctrl+Enter, Ctrl+Shift+Enter, Escape, Ctrl+Z, Ctrl+Y), which the shell resolves before the envelope (EV4.3). The
   source route (C5) runs a built-in tool whose program proposes the literal word. The headless key route is source
   evidence; the actual key route is windows evidence (PHS.23).
5. **Runner and records (PHS.65).** As in LP2: `cargo test --locked --offline` in `rust/` through
   `tests/test_language_rust.py`, and the oracle loop through `tests/test_lp3_loop.py`, both listed in
   `tools/ci/run_headless.py`. A test that implements part of a conformance test names the ID and the part, and the ID
   counts as run only when its whole definition runs (PHS.62, PHS.63). The LP2-indexed tests the command layer serves
   (CT-KN5, CT-KN7-04, CT-KN7-05, CT-ONE-03 to CT-ONE-22, CT-LI-10) stay LP2's. The evidence record is the CI run of the
   merged commit, named in the pull request.
6. **Critical path.** `c600-command` holds the protection recheck and undo under protection, so each change to it gets
   one read by a fresh Claude subagent on a smaller model before commit, given the diff and the acceptance check
   (`AGENTS.md`, "Codex minimum").
7. **The representations tool** is LB6.5's representation, work-set and filter row, a view aid: filter preview and
   apply, the effect-scope view, work-set save, and structure comparison and layering over LB4's layouts and atlases,
   with the map property checks and linked selection. Its headless half needs no renderer.
8. **Order of value.** The 600-cell-Full vertical path first (C1, C5), then the representations tool (C6); the other
   source and synthetic tests that need no renderer run in parallel crates (C2 to C4).

## 4. Milestones

One pull request per milestone; each names the checks that ran.

| Milestone | Content | Depends on | State |
| --- | --- | --- | --- |
| C1 | Command layer, shell tokens and key routes; O01 to O07 through it on two routes, with undo and redo | M4 (on main) | Built; this plan's pull request |
| C2 | Headless display contract (`c600-display`); views and their budgets (`c600-view`) | M2 and M3 (on main) | Built |
| C3 | The runtime in its own process (`c600-runtime`) | M2 and M3 | Built |
| C4 | Module closure (`c600-module`) | M2 and M7 (on main) | Built |
| C5 | Source route and the 600-cell-Full tool (`c600-tools`) | C1 | Built in part (section 7) |
| C6 | Representations tool, headless half | C2, C5 | Built in part (section 7) |
| C7 | Durable receipts and undo through the session store | M6 (on main), C1 and LP2's KN5 implementation | Built in part (section 8) |
| C8 | The jumbling tool | M5 on main | Waits |
| C9 | The on-screen half and the owner-machine acceptance runs | The renderer freeze; the owner's runs | Waits |
| C10 | LP3 exit: H-08 freeze, the whole index on one frozen candidate, the owner's sign-off | C1 to C9 | Waits |

## 5. C1: the command layer and the oracle loop

Crate `rust/c600-command`, with the loop runner `c600-lp3-oracle`. Every change to a session goes through the layer's
gates (KN5.1). A word commit runs chapter 08 section 7's ten steps in order; undo and redo take LI.9's hint and derive
through the kernel under the protection in force (LI.26); every invocation gets a receipt, and a retry finds it, also
across a restart (KN5.21, KN5.23, LI.30). The shell mints the tokens from a per-launch secret the runtime never receives
(KN5.14, KN5.15). Acceptance:

1. `cargo test --locked --offline` and `cargo fmt --all --check` pass in `rust/`, in CI through
   `tests/test_language_rust.py`. The crate's tests cover the command-layer parts of CT-KN5-01, CT-KN5-03, CT-KN5-04,
   CT-KN5-06, CT-KN5-07, CT-KN5-09, CT-KN5-11, CT-KN5-12, CT-KN7-04, CT-KN7-05, CT-ONE-03, CT-ONE-04, CT-ONE-05,
   CT-ONE-11, CT-ONE-20, CT-ONE-21, CT-ONE-22 and CT-LI-10, each naming its part.
2. `tests/test_lp3_loop.py` drives O01 to O07 through the command layer on the command and key routes:
   `tools/oracle/compare.py` reports every step equal with no omitted fields (3,201 steps per route); every accepted
   step commits once; an independent case undoes each commit to its initial state; a chain case undoes and redoes
   every commit onto the recorded states; the two routes write byte-identical traces and receipts. A tampered trace
   fails at its step, and a case with another model identity is refused without a trace.
3. The runtime reaches at most a preview: the 19 commit and app rows and any token it carries are refused (ADR-L02).
4. A fresh Claude subagent on a smaller model reads the crate with this acceptance check, and its blocking findings are
   fixed before the pull request.
5. No file under `docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Stop condition: C1 ends when items 1 to 5 pass in CI and its pull request is merged. It stops earlier, and the owner is
told, if a step needs a frozen-file change, an open contract or a product dependency.

## 6. C2 to C4: the headless pieces in parallel

Each piece is built in its own worktree from main at `b5d3c41a`, with a seven-part packet that names its allowed files,
acceptance check and stop condition.

- C2, display (`rust/c600-display`, branch `claude/lp3-display`), built: CT-RPI-02, CT-RPI-04, CT-RPI-05, CT-RPI-09,
  CT-RPI-10, CT-RPI-11, CT-RPI-27, CT-RM-03 and CT-RM-04, CT-RPI-08's 600-cell-Full rows, the stale half of CT-RPI-12
  and the 600-cell-Full half of CT-RM-07. CT-RPI-08's jumbling rows wait for M5 (C8).
- C2, views (`rust/c600-view`, branch `claude/lp3-view`), built: CT-ONE-30, CT-RM-02, CT-RPI-03, CT-RPI-06,
  CT-RPI-07, CT-RPI-36 and CT-RPI-37, the limit half of CT-RPI-22 and rows 1, 2, 7 and 9 of CT-RM-09.
- C3 (`rust/c600-runtime`, branch `claude/lp3-runtime-process`), built: the runtime in a process of its own; 8 of
  DET.65's 40 traces of CT-RTE-04 and the headless fault half of CT-RTE-10. C5's host bridge makes it a client of the
  command layer (ADR-L02). CT-RTE-04's other 32 traces wait for the IR draft's features (LP2).
- C4 (`rust/c600-module`, branch `claude/lp3-module-closure`), built: module closure, its lock and the approval bound
  to the closure digest; CT-RTE-08's source leg and 36 of LX-3's 75 frozen keys of CT-RTE-07 on 600-cell-Full.
  CT-RTE-08's persisted words come with C7; CT-RTE-07's other keys wait for M5, C7, the IR draft and C5.
- C2, display next (`rust/c600-display`, branch `claude/lp3-c2-display-next`), built: CT-RPI-14 over the drafted
  chart repertoire (RPI.34 is open), CT-RPI-18 on the CPU reference, CT-RPI-19's Windows leg, CT-RPI-22's chart half
  (so CT-RPI-22 runs whole) and the headless part of CT-RPI-32. The rest of CT-RPI-32 needs the stage module's checker
  and a backend.
- C2, views next (`rust/c600-view`, branch `claude/lp3-c2-view-next`), built: CT-RPI-23, whose two parts both run,
  the counter half of CT-RPI-26 and the admission semantics of CT-EV3-08. Engine epochs now bind revisions, bundles
  and output, and a scheduled recomputation can neither open nor close a view.
- Next, in C2's crates: nothing more runs now. CT-RPI-26's gate half and CT-EV3-08's measured half wait for C9, and
  CT-RM-09's rows 3 to 6, 8 and 10 need a stage checker or a device.

Acceptance for each piece: `cargo test --locked --offline` and `cargo fmt --all --check` pass in `rust/`; each listed
test's part exists and names its part; nothing needs a renderer, a GPU or a timing; no file under
`docs/architecture/1.0/language/` changes, and the eight language checkers still pass. What does not finish is listed
in `rust/README.md` as not built. CT-RPI-19 and CT-RTE-07 also need a Linux run for their records; CI runs Windows.

Stop condition: each piece ends when its acceptance passes and its pull request is merged. It stops earlier, and
reports, if it needs a frozen-file change or a change to an LP2 crate.

## 7. C5 and C6: the source route and the representations tool

C5, crate `rust/c600-tools`. The 600-cell-Full tool is a `.6cs` module in edition `6cs-draft-1`, checked in the solving
profile (it prepares previews and reads no state: LG3.13, BND.39), with its class declared (BND.37). It runs on
`c600-run` through a host bridge onto the command layer: its proposal reaches `operation.preview` as a runtime
invocation, and only the shell commits (ADR-L02). It runs in-process until C3 is on main, and its records say which
host ran it. Acceptance:

1. The source route replays O01 to O07. Each step's literal word, the kernel's state-free expansion of its recipe,
   enters the tool as a pinned absolute argument; `tools/oracle/compare.py` reports every step equal, the traces equal
   the command route's byte for byte, and the receipts agree on outcome, literal digest, error code and the revision
   after, differing only in their envelopes (command ID, caller, request digest).
2. The parts of CT-LB2-02 (the Net half), CT-LB2-03 (the oversize refusal), CT-LB3-01 and CT-LB3-07 (the checker
   halves), CT-LB6-03 (the 600-cell-Full tool), CT-BND-15 (the 600-cell-Full tool), CT-BND-19, CT-EV3-01 to CT-EV3-04
   with the tool's views, CT-EV4-02, CT-EV4-04 (the append, preview and commit legs) and CT-ONE-29 that the tool and
   the bridge make reachable, each naming its part.
3. A change to `c600-command` gets C1's fresh read; C1's item 5 holds.

- C5 (`rust/c600-tools`, branch `claude/lp3-source-route`), built: item 1 (compare.py equality on all 3,201 steps,
  traces byte-identical to the command route's, and receipts that agree in all 13,553 pairs apart from their
  envelopes); item 3 (C5 changed no file in `c600-command`); and of item 2, the 600-cell-Full tool parts of CT-LB6-03
  and CT-BND-15, the headless part of CT-ONE-29 and the command-layer parts of CT-BND-07 and CT-BND-11. The tool runs
  in process.
- C5, input router (`rust/c600-input`, branch `claude/lp3-input-routing`), built: of item 2, CT-EV4-02 and CT-EV4-04's
  append, preview and commit legs and its busy leg, on synthetic events through the shell's public API; ahead of LP4,
  CT-EV4-06's synthetic half. CT-EV4-04's twist leg waits for KN3.7's exact rotations (section 2).
- C5 next, the process host (`rust/c600-tools`, branch `claude/lp3-c5-process`), built: the tool on C3's runtime process
  (`c600-runtime`'s child), with the same boundary and bridge in the caller. `c600-lp3-source --host process` replays
  O01 to O07 with traces byte-identical to the in-process host's, and every record names its host; four O07 steps whose
  words have a letter outside 1 to 1200 in magnitude cannot cross the runtime's wire, so they have no receipt on the
  process host and their records keep the kernel's refusal. Of item 2: CT-LB2-02's Net half; CT-LB2-03's oversize
  refusal on both routes, with chunking on the command route; CT-LB3-07's checker half for outcome tables and named-word
  checks. CT-LB3-01's checker half and CT-LB3-07's recommendation rule need `c600-check` to decide them (LP2). A test
  confirms a lead on KN5.21 to KN5.23 (a refused forgery takes a genuine command ID's retry record), sent to LP2's KN5
  implementation.
- C5 next, the tool's views (`rust/c600-tools`, branch `claude/lp3-c5-views`), built: CT-EV3-01, CT-EV3-02 and
  CT-EV3-04, whole for the tool's view; CT-EV3-03, whole, with its imported half on a fixture import record, since the
  Rust layers have no import row; CT-BND-19's part for the tool's views; CT-ONE-29's user-stage part at the host's
  adapter. The shell's own refusal of a user-stage frame waits for LP2's KN5 implementation, which adds a
  presented-frame input to `present`; CT-BND-19's live-twist part waits for KN3.7's exact rotations (section 2).
- Next, in C5's crate: nothing more runs now. The library entries wait for the IR draft (below), the jumbling halves for
  M5 (C8), and the parts above for LP2.

CT-LB1-01, CT-LB1-03, CT-LB1-06, CT-LB2-01 and CT-BND-18 test library entries, which are language code (LB1.1). They
need collection types and word operations (concatenation, inverse, powers) that the IR draft `c600-ir-draft-1` does
not have yet, so they wait for LP2 (section 2).

C6, the representations tool, headless half (selection 7). Acceptance: CT-LB4-02, the 600-cell-Full half of CT-LB4-03,
the admission and CPU half of CT-LB4-04, the CPU reference of CT-LB4-01, the counter half of CT-EV3-06, the
view-cannot-write half of CT-EV3-07 and the representations parts of CT-LB6-03 and CT-BND-15, each naming its part;
its rows meet C5's item 1 through the source and command routes; C1's item 5 holds.

- C6 (`rust/c600-repr`, branch `claude/lp3-c6-repr`), built: CT-LB4-02, whole (synthetic); the 600-cell-Full half of
  CT-LB4-03; the admission and CPU half of CT-LB4-04; CT-LB4-01's CPU reference (a declared domain decided by an exact
  region test and section membership on the posed vertices, boundary cases included, a chart whose domain fails inside
  refused, and domain and injectivity each established by their own check); the counter half of CT-EV3-06; the
  view-cannot-write half of CT-EV3-07 at the command layer; and the representations parts of CT-LB6-03 and CT-BND-15.
  The tool is a `.6cs` module with five entries admitted under the built-in column, with a Rust manifest of the class
  `view_aid`; the class review covers every exported function and only the source bytes the manifest names. C6 changed
  no file in `c600-command` (C1's item 5).
- C6, not built: the six rows, so they do not meet C5's item 1 yet. The command layer refuses them as not built, and LP3
  changes no layer code until KN5's one implementation lands; `c600_repr::rows` writes what each needs from the layer
  (section 2), and `set.save` has no entry until the IR draft has a list type. Also not built: the checkers for
  homomorphism, embedding, isometry and full-dimensional injectivity (KN3's exact polytope operations, LP2), the stage
  checker and every device step (the renderer freeze, C9), pose levels and off-lattice relations (C8's next piece, on
  M5's kernel), the relationship strip, and a BM5 review record, which the tool gets when it ships.

Stop condition for both: each ends when its items pass in CI and its pull request is merged. Each stops earlier under
C1's conditions, or when a tool needs a kernel row LP2 has not built.

## 8. C7 to C10: what waits

- **C7, durable receipts on M6's store.** KN5 gets one implementation, owned by LP2 under PHS.18 (integrator,
  16 November 2026); the LP2 integrator placed it in `c600-command`, with `c600-session` implementing its `Store`
  trait, and until it lands LP3 changes neither `c600-command`'s layer, token, receipt, store and shell code nor
  `c600-session`. Built (`rust/c600-durable`, test-only), on M6's public API: CT-RTE-08's persisted-word case, its
  durable leg; CT-EV3-07's durable action order and a view part on M6's stand-ins; CT-LB2-03's work-sheet part.
  Waits for LP2's KN5 implementation: durable receipts and undo through the command layer, KN5.26's admission of
  record words, EV3.2's view rows and LB7.2's work-sheet structure.
- **C8, after M5 is on main.** The jumbling tool, with Jumble on in the command layer and oracle equality against M5's
  fixtures; CT-LB1-02, the jumbling halves of CT-LB1-03 and CT-LB1-06, CT-LB3-01 to CT-LB3-07, the pose levels of
  CT-LB4-03, CT-LB6-03, CT-BND-15, CT-BND-17, CT-EV3-06, CT-RPI-12 and CT-RM-07.
- **C9, after the renderer freeze, on the owner's machine.** The windows tests CT-ONE-26, CT-RPI-13, CT-RPI-21,
  CT-RPI-24, CT-RPI-31, CT-RPI-33, CT-RPI-34, CT-RM-05, CT-RM-06, CT-RM-10, CT-BND-20 and CT-RTE-10; the performance
  tests CT-KN7-06, CT-LG4-09, CT-EV1-06, CT-ONE-25, CT-RPI-26, CT-RTE-11 and CT-RTE-12; the renderer halves of
  CT-LB4-04 and CT-EV3-08. The runs follow PHS.64 on one frozen candidate, with their bars unchanged, and are done
  together (owner, 16 November 2026).
- **C10, the exit.** H-08 freezes (PHS.53), with the draft codes registered under PHS.52; every acceptance item has
  evidence on one frozen candidate; the 82 tests run whole; the owner signs off.

## 9. Section 15.4's 82 tests

"Now" means the whole test can run without the owner's machine, the renderer freeze, M5 or a missing kernel row.
28 tests can run whole now (CT-RPI-19 also needs a Linux run for its record), 25 have a part that can run now, and 29
wait entirely. The milestone is in parentheses.

| Test | Class | Whole run | Part now |
| --- | --- | --- | --- |
| CT-KN7-06 | performance | Owner runs: charter E2 under a running program job (C9) | - |
| CT-LG4-09 | performance | Owner runs: LX-2 (C9) | - |
| CT-LB1-01 | source | Collection types in the IR draft and the F2 and F5 number kinds, for the library code (LP2) | - |
| CT-LB1-02 | source | M5: a blocked cap (C8) | - |
| CT-LB1-03 | source | Collection types and word operations in the IR draft (LP2); M5 (C8) | - |
| CT-LB1-06 | source | Collection types and word operations in the IR draft (LP2); an owner answer on LG2.18's word keys; M5 for jumbling letters (C8) | - |
| CT-LB2-01 | source | Collection types and word operations in the IR draft, for the library's recipes and reports (LP2) | - |
| CT-LB2-02 | source | The kernel's Strict protection (LP2) | The Net half (C5) |
| CT-LB2-03 | source | Streaming words, KN2.10 (LP2) | The oversize refusal (C5); work sheets (C7) |
| CT-LB2-04 | source | The kernel's `positions` row, KN2.4 (LP2) | - |
| CT-LB3-01 | source | M5 (C8); for the checker half, typed checkpoint and sticker-test results in `c600-check` (LP2) | - |
| CT-LB3-02 | source | M5 (C8) | - |
| CT-LB3-03 | source | M5 (C8) | - |
| CT-LB3-04 | source | M5 (C8) | - |
| CT-LB3-05 | source | M5 (C8) | - |
| CT-LB3-06 | source | M5 (C8) | - |
| CT-LB3-07 | source | M5 (C8); LB3.5's recommendation rule in `c600-check` (LP2) | The checker half for outcome tables and named-word checks (C5) |
| CT-LB4-01 | source | KN3's exact polytope operations in the kernel (LP2) | The CPU reference (C6) |
| CT-LB4-02 | synthetic | Now (C6) | - |
| CT-LB4-03 | source | M5: pose levels (C8) | The 600-cell-Full half (C6) |
| CT-LB4-04 | source | The renderer freeze (C9) | Admission and the CPU half (C6) |
| CT-LB6-03 | source | M5 (C8) | The 600-cell-Full and representations tools (C5, C6) |
| CT-EV1-06 | performance | Owner runs: LX-4 (C9) | - |
| CT-EV3-01 | synthetic | Now (C5) | - |
| CT-EV3-02 | synthetic | Now (C5) | - |
| CT-EV3-03 | source | Now (C5) | - |
| CT-EV3-04 | synthetic | Now (C5) | - |
| CT-EV3-06 | source | M5: off-lattice relations (C8) | Unchanged frames compute no relation (C6) |
| CT-EV3-07 | source | An explicit workspace command through the command layer, in the shared `session.undo` order: KN5's one implementation (LP2; KN5.20, EV4.15) | A view cannot write, at the command layer (C6); the durable action order on M6's store (C7) |
| CT-EV3-08 | source | The renderer freeze: RM.39's measured ceilings (C9) | The admission semantics (C2) |
| CT-EV4-02 | synthetic | Now, on the draft keymap (C5) | - |
| CT-EV4-04 | synthetic | KN3.7's exact rotations in the kernel, for the twist leg (LP2) | The append, preview and commit legs (C5) |
| CT-ONE-25 | performance | Owner runs: charter E2 (C9) | - |
| CT-ONE-26 | windows | The renderer freeze (C9) | The token binds the presented preview (C1) |
| CT-ONE-29 | source | The shell's own refusal of a user-stage frame: a presented-frame input to `present` (LP2's KN5 implementation) | The headless part and the user-stage part at the host's adapter (C5) |
| CT-ONE-30 | source | Now (C2) | - |
| CT-RPI-02 | source | Now (C2) | - |
| CT-RPI-03 | synthetic | Now (C2) | - |
| CT-RPI-04 | source | Now (C2) | - |
| CT-RPI-05 | source | Now (C2) | - |
| CT-RPI-06 | source | Now (C2) | - |
| CT-RPI-07 | synthetic | Now (C2) | - |
| CT-RPI-08 | source | M5: the jumbling rows (C8) | The 600-cell-Full rows (C2) |
| CT-RPI-09 | source | Now (C2) | - |
| CT-RPI-10 | synthetic | Now (C2) | - |
| CT-RPI-11 | synthetic | Now (C2) | - |
| CT-RPI-12 | source | M5 (C8) | The stale half (C2) |
| CT-RPI-13 | windows | The renderer freeze (C9) | - |
| CT-RPI-14 | synthetic | Now (C2) | - |
| CT-RPI-18 | synthetic | Now (C2) | - |
| CT-RPI-19 | source | Now, with a Linux run (C2) | - |
| CT-RPI-21 | windows | The renderer freeze (C9) | - |
| CT-RPI-22 | source | Now (C2) | - |
| CT-RPI-23 | source | Now (C2) | - |
| CT-RPI-24 | windows | The renderer freeze and M5 (C9) | - |
| CT-RPI-26 | performance | Owner runs: charter E2 and the renderer gate (C9) | The counter half (C2) |
| CT-RPI-27 | source | Now (C2) | - |
| CT-RPI-31 | windows | The renderer freeze (C9) | - |
| CT-RPI-32 | source | The stage module's checker (RPI.69 to RPI.71) and a backend (C9) | The headless part (C2) |
| CT-RPI-33 | windows | The renderer freeze (C9) | - |
| CT-RPI-34 | windows | The renderer freeze (C9) | - |
| CT-RPI-36 | source | Now (C2) | - |
| CT-RPI-37 | synthetic | Now (C2) | - |
| CT-BND-15 | source | M5: the jumbling tool (C8) | The 600-cell-Full and representations tools (C5, C6) |
| CT-BND-17 | source | M5: the repair list (C8) | - |
| CT-BND-18 | source | Collection types and word operations in the IR draft, for setup search (LP2) | - |
| CT-BND-19 | source | A live twist's release, which needs KN3.7's exact rotations (LP2) | The tool's views (C5) |
| CT-BND-20 | windows | The renderer freeze (C9) | A held chord commits once; the runtime gets no token (C1) |
| CT-RTE-04 | source | Child scopes, loops, text order, byte decoding, exact multiplication, streaming and LP1's region executor in the IR draft (LP2) | 8 of DET.65's 40 traces (C3) |
| CT-RTE-07 | source | M5's jumbling keys (C8), M6's store keys (C7), the keys the IR draft cannot express (LP2), the budget keys that need a price mapping, host events in a loop or the command layer (C5), and a Linux run | 36 of LX-3's 75 frozen keys on 600-cell-Full (C4) |
| CT-RTE-08 | source | Now: module closure (C4); persisted words (C7) | - |
| CT-RTE-10 | windows | Owner runs: LX-2 (C9) | The headless fault half (C3) |
| CT-RTE-11 | performance | Owner runs: LX-2 (C9) | - |
| CT-RTE-12 | performance | Owner runs: LX-4 (C9) | - |
| CT-RM-02 | source | Now (C2) | - |
| CT-RM-03 | source | Now (C2) | - |
| CT-RM-04 | source | Now (C2) | - |
| CT-RM-05 | windows | The renderer freeze (C9) | - |
| CT-RM-06 | windows | The renderer freeze (C9) | - |
| CT-RM-07 | source | M5 (C8) | The 600-cell-Full half (C2) |
| CT-RM-09 | source | A user-stage checker and a device, for rows 3 to 6, 8 and 10 (C9) | Rows 1, 2, 7 and 9 (C2) |
| CT-RM-10 | windows | The renderer freeze (C9) | - |

## 10. Stop condition for LP3, and questions to the owner

LP3's offline work stops and reports when every remaining milestone waits for M5, a kernel row, the IR draft's
collection types, word operations and the features CT-RTE-04's traces need, the renderer freeze, an owner run or an
owner answer, or when an implementation would change a frozen file (PHS.52). A failing acceptance item blocks LP3's
exit (PHS.63); nothing is cut, and no bar moves.

Questions are batched (owner, 16 November 2026): at most one card per milestone, each with a recommended option.

- C5: LG2.18's word keys are open (the LP2 plan and `rust/README.md` record it), and CT-LB1-06 needs a key once the
  IR draft has collection types and word operations. Recommended: LP3 uses LB1.10's proposed `trace-key/1`, the
  lexicographically least topological word, as a draft key for 600-cell-Full words; it freezes with the IR, and the
  jumbling letters wait for M5. The alternative keeps the keys open and leaves CT-LB1-06 waiting.
- C1 to C4: none. The command table's choices are drafts that freeze at LP3.
- C9: none before the renderer freeze; the owner-machine runs follow PHS.64 and are done together.
- C10: at the exit, H-08's freeze and the registration of the draft codes in chapter 13 (PHS.52, PHS.53).
