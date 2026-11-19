# Language LP3 plan: the first closed loop

Status: plan of 16 November 2026, written when LP3's offline work started beside LP2's remaining milestones, and brought
up to date on 18 November 2026 with LP2's merged pull requests up to 272, the renderer freeze (ADR-R01), the C9 line's
pull requests 240, 244, 245, 248, 252, 259, 261, 263, 267 and 268 and the owner's decisions of that day
([decision](../../wiki/decisions/owner-decisions-2026-11-18.md),
[route B](../../wiki/decisions/owner-decisions-2026-11-18-route-b.md)), and on 19 November 2026 with LP2's pull request
277 and C8's pull requests 278 and 288. It records the integrator's selections, and the
owner may overrule any of them. It is no ADR and freezes nothing: the command table stays a draft until it freezes at
LP3 (PHS.53), and the IR until its own freeze (PHS.49).

Inputs: chapter 12 sections 5 and 15.4 (PHS.22 to PHS.25, PHS.61 to PHS.67); chapter 01 KN5; chapter 08 section 7;
the [layer interfaces](layer-interfaces.md) (LI.5, LI.6, LI.9, LI.26, LI.30); H-08's stored
[command table](command-table.json); ADR-L02; chapter 03 (LB1 to LB6); chapter 10 and the renderer feature matrix; the
owner decisions of 14 to 17 November 2026; [ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md),
the renderer selection; the [LP2 plan](language-lp2-plan.md).

Evidence kind for everything this plan produces without the owner's machine: source and fixture, and synthetic where a
test's class says so. Nothing here is Windows interaction, DirectX, input, long-session or performance evidence; the
timings the checks print are informational. The windows and performance tests run on the owner's machine under PHS.64.

## 1. What LP3 delivers

PHS.22: three built-in tools written in the language above the kernel, one for 600-cell-Full, one for jumbling and one
for representations, each demonstrating the complete loop. Its four products:

| Product | Without the owner's machine | Waits for |
| --- | --- | --- |
| Three-tool vertical path: source, check, run, view nodes, engine preview, explicit user commit, durable receipt and undo | 600-cell-Full: preview, commit and undo through the command layer with oracle equality (C1), the source route (C5), view nodes (C2, C5), durable receipts and undo in part (C7). Representations: the headless half (C6). Jumbling: the tool's source and admission, LB3's kernel halves, and presses and undo with Jumble on in the command layer (C8) | The rest of durable receipts and undo through the command layer: KN5's later stages (LP2; C7, section 8). The jumbling tool's entries through the command layer with Jumble on (LP2's pull requests 212, 217, 218, 220 and 254), which C8 has not run yet (section 8) |
| Matching command and key routes, kernel-row keys on ONE.48's direct path | The command and key routes with byte-identical traces and receipts (C1); the source route (C5) | The actual key route on Windows (C9) |
| Active program views under ONE.11, ONE.12 and ONE.64 | Revision-bound output, latest-wins recomputation and stale-state data, headless (C2) | On screen, on the selected renderer (C9) |
| Further views and user stages | `view.open`, `view.close` and RPI.82's budget, headless (C2) | User vertex and fragment stages on the selected renderer, advertised only after RPI.79's run (C9) |

PHS.23's six acceptance items:

| Acceptance item | Without the owner's machine | Waits for |
| --- | --- | --- |
| Same result through all routes | Command and key routes (C1); source route (C5); five of the representations tool's six rows through the command and source routes, on a test host whose stand-in engine answers the two snapshot reads with the state number (C6, section 7) | `filter.apply` through the same two routes: the layer serves it since LP2's pull request 256 and no LP3 test runs its entry yet (C6, section 7); the actual key route (C9); the jumbling tool's entries through the command and source routes with Jumble on in the command layer, which C8 so far presses only for oracle equality and CT-LB4-03 (section 8) |
| Oracle equality at every committed step | 600-cell-Full: O01 to O07, 3,201 steps per route, with undo and redo (C1). Jumbling: M5's W-J fixtures through `jumble.on` and `jumble.twist.menu`, every press beside M5's engine, the first 130 records of each journal by default and the whole journals on request (C8); LP2's `c600-jumble-route` compares the same layer with J1, with undo and redo, on the first 70 records (pull requests 212, 217, 218, 220 and 254) | Jumbling: protected pieces and free mode through the command layer (section 8) |
| Presented-preview binding in the selected renderer | The token binds the presented preview, headless (C1) | The on-screen half on the selected renderer, on Windows (C9) |
| Charter E2 with the tools' views active | Nothing: a timing here would be no evidence | The owner-machine runs (C9) |
| Stage and view contract on the selected renderer | The headless display contract (C2) | The on-screen half on the selected renderer, on Windows, and RPI.79's gate run (C9) |
| Decisive LX-1 to LX-4 checks | The headless parts: the execution-contract mapping and LX-2's faults (C3), module closure (C4) | The owner-machine runs (C9). A failing LX-1, LX-2 or LX-4 run reopens LQ9; a failing LX-3 check goes to the owner |

LP3 exits only when every acceptance item has evidence on one frozen candidate and the 82 tests of section 15.4 run
whole: 50 source, 13 synthetic, 12 windows and 7 performance (PHS.62, PHS.63, PHS.66). LP3 depends on LP2, frozen H-08
and the renderer freeze for its on-screen half, and headless integration does not pass the on-screen acceptance early
(PHS.24). The owner signed the renderer freeze on 16 November 2026
([ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md); [freeze
record](../../wiki/decisions/renderer-freeze.md)): the renderer is S-A2, the S-B Direct3D 12 drawing core as a shared
native library hosted by Godot 4.7.2 .NET, so the on-screen half may start (C9, section 8). The owner started it on 18
November 2026 as a line of its own.

## 2. What may start now and what waits

| Work | State | Waits for |
| --- | --- | --- |
| Command layer, shell tokens and key routes; the oracle loop for 600-cell-Full | Built (C1) | Nothing; the table freezes at LP3 |
| Headless display contract; views and their budgets | Built (C2) | Nothing |
| The runtime in its own process | Built (C3) | Nothing: route RT1 (LQ9) |
| Module closure | Built (C4) | Nothing |
| Source route and the 600-cell-Full tool | Built in part (C5, section 7) | The parts section 7 names for LP2; the jumbling halves are C8's (section 8) |
| Representations tool, headless half | Built in part (C6, section 7): five of its six rows served through the command and source routes, on a test host whose stand-in engine answers the two snapshot reads with the state number | LP3's next steps (section 7): `filter.apply` through the test host, with an entry that takes the layer's `FilterPreviewRef` (LP2's pull request 256); argument blocks for a running program's query and view jobs (pull request 264); host rows for the two snapshot reads, on LP2's structure rows (pull request 262). LP2 built the `CanonicalObjectRefs` form (pull request 269). Not built: a record and an exact-set digest for each saved set |
| Durable receipts and undo, CT-RTE-08's persisted words, work sheets | Built in part, through the command layer over `JournalStore` and on M6's store (C7, section 8): the part blocks the layer keeps for C5's reader (LP2's pull request 272), and LB7.2's fresh exact analysis of a reused work sheet (`worksheet.compare`, with LB2's gains and losses) and macro versions (LP2's pull request 277). KN5.26's admission matrix L1 and L2 runs through the layer in LP2's tests (pull request 270). M6's session lock covers `JournalStore` (pull request 195) | LP3's next step (section 8): `c600-view`'s view model serves EV3.2's view rows (pull request 271, landed through 272). From LP2: a session profile (BND.14, BND.15), and reading a position role's occupant again at a work-sheet reuse |
| The jumbling tool and every jumbling half | Built in part (C8, section 8) | LP2's exposed-face geometry for J10, a jumbling word key, and a job that names its cap through the command layer (section 8). Built and not used by C8 yet: with Jumble on in the command layer, `jumble.off`, the refusals, redo, protected pieces and free mode (pull requests 212, 217, 218, 220 and 254). C8 uses the IR draft's F2 and F5 (pull request 222) for LB3's ν and drafted correction-set entry as language code (`jumble_nu`, pull request 295), `jumble-locality/1` with `trace_key` on menu words (pull request 251) for CT-LB1-06's jumbling half, and Jumble on in the command layer for oracle equality and CT-LB4-03 at a later revision. M8's observations and levels are built (pull requests 176 and 182); C8 uses the slot action of K⁺, the end tests, the projection from poses to labels, a whole word's protection for CT-LB3-02 and the repair list; LP2's kernel pieces for C8 (pull request 190): the slot action with its frame check, the rotation plane and the label-level protection of named pieces; and LP2's ν and correction catalogue (pull requests 201 and 206) for CT-LB3-04, CT-LB3-01's ν case and CT-LB3-07's correction part (section 8) |
| Tests that read the kernel rows LP2 built in pull requests 170 and 176: Strict protection, streaming words (KN2.10), `positions` (KN2.4), KN3's exact rotations and exact vertices | Built in part (C5 library: CT-LB2-02's Strict half at the kernel, and CT-LB2-04 whole over `positions` and a `labels` host row; C5: its Strict half through the tool's route, on the command layer's Strict mode of LP2's pull request 198, and the twist keys' exact requests through `twist.apply`; C6: CT-LB4-01's checks on the polytope operations); the others can start | What the kernel alone does not give: host rows beyond `positions` and `labels` in the tool's bridge, LP3's to add, among them the two snapshot reads of `structure.compare` and `filter.preview`, on LP2's structure rows (pull request 262; the table below); LB7.2's work sheets are in the command layer (pull request 232) |
| The on-screen half | Started as a line of its own (owner, 18 November 2026; C9, section 8) | Its components, which that line builds; the owner's runs |
| Charter E2, the decisive LX-1, LX-2 and LX-4 runs, LX-3's platform coverage, RPI.79's gate run and the W-J runs on the selected renderer | Waits | The owner's machine, all run together (owner, 16 November 2026; ADR-R01.3); LX-4's time box is 4 hours (owner, 18 November 2026); route B's three checks join them ([route B](../../wiki/decisions/owner-decisions-2026-11-18-route-b.md)) |
| CT-LB1-06's word keys | Answered: `trace-key/1` (owner, 17 November 2026, [decision](../../wiki/decisions/owner-decisions-2026-11-17-c5-evidence.md)); the 600-cell-Full half is built (LP2's pull request 214), and so is the jumbling half's relation, `jumble-locality/1`, with `trace_key` on menu words (LP2's pull request 251), which C8 pins in `c600-jumble-tool`, where the jumbling half's library entry `menu_traces` runs (C8) | LG2.18's jumbling word key, for "Word keys remain separate" on the jumbling side (LP2) |
| Library entries as language code (LB1.1): collections, exact linear algebra, word construction, word and trace keys, setup search | Built in part (C5 library, section 7): LB1's literal words and LB2's exact completion on 600-cell-Full; the jumbling entries on menu words (C8, section 8), whose rows a test host serves | Nothing from LP2 for the next entries: menu words (pull request 210), `trace-key/1` (pull request 214), F2 and F5 (pull request 222) and loops (pull request 234) are in the IR draft. Entries over kernel rows, such as LB2's star and recipe construction and its Net and Strict protection, need a host row for each kernel row they read; the tool's bridge answers the IR draft's snapshot read with an argument for `positions` (LP2's pull requests 199 and 226; the IR freezes at PHS.49) and `labels` (C5 library) only, and LP3 adds the others, among them LP2's structure rows (pull request 262). The other entries are not written yet |

LP2 deliverables that LP3's tests wait for. The LP2 line confirmed on 16 November 2026 that each is LP2 work: section
15.3 lists its tests, and PHS.18's product row "Kernel API over both puzzles" and the LP2 plan's M8 row ("section 15.3
runnable") cover the kernel rows. LP3 builds nothing in LP2's crates. Each row says what LP2 has built (pull requests
170, 176 and 185) and what the LP3 tests in its last column still wait for.

| LP2 deliverable | Rule | LP2 tests | LP3 tests that wait |
| --- | --- | --- | --- |
| Strict protection on 600-cell-Full. Built in the kernel (pull request 170: `strict_protection`, `derive_strict`, `replay`) and in the command layer (pull request 198: `protection.policy.set` switches a session between Net and Strict, and the runtime may not invoke it) | KN2.4, KN2.7 | CT-LB2-02's Strict half in the command layer (`c600-command/tests/strict_policy.rs`); CT-KN2-03 and CT-LG2-12 use the `protection` row | None: CT-LB2-02's Strict half runs through the tool's route (C5) and at the kernel (C5's library) |
| Streaming words. Built in the kernel (pull request 170: `chunks`, `replay`, `replay_under_ceiling`), with a compressed word's part list (pull request 185); LB7.2's work sheets in the command layer (pull request 232: `worksheet.save`, `worksheet.load`; pull request 277: `worksheet.compare` and macro versions) | KN2.10 | CT-KN2-07, CT-LG2-14 | None waits: C7 runs CT-LB2-03's named work-sheet reuse and comparison through the layer |
| The `positions` row. Built in the kernel (pull request 170: `Kernel::positions`) and served to a program's snapshot read with an argument through the tool's bridge (pull request 226: `c600_tools::kernel_rows`, with `CommandLayer::positions`) | KN2.4 | CT-LB2-04's host part (`c600-tools/tests/positions_row.rs`) | None waits for the row: CT-LB2-04 runs whole with LB2's exact-completion entry, LP3's library code, over this row and a `labels` host row (`c600-tools/tests/exact_completion.rs`; section 9) |
| Exact rotations and `twist.apply` (amendment A6). The exact requests are built (pull request 176: `c600_jumble::rotation`, `c600_kernel::request`), and so is `twist.apply` in the command layer (pull request 207). Not built: the shell's gesture producer (CT-KN3-06) | KN3.4, KN3.7 to KN3.9 | CT-KN3-04 to CT-KN3-06, CT-LG2-16 | None: CT-EV4-04's twist leg and CT-BND-19's live-twist part run in `c600-input` (C5) |
| Exact polytope operations. Built: KN3.3's exact double description (pull request 176: `c600_jumble::dd`, the exact vertices of a bounded region) and the other operations with the exact core of LB4.1's map property checks (pull request 209: `c600_jumble::polytope`) | KN3.3 | CT-KN3-01, CT-KN3-03 | None; CT-LB4-01's injectivity, embedding, isometry and homomorphism checks run on them (C6) |
| The F2 and F5 number kinds. Built in the kernel (pull request 201: `c600_kernel::exact`), with ν (`c600_jumble::nu`, pull request 201) and the correction catalogue with `correction_set_check` (`c600_jumble::corrections`, pull request 206), and as number kinds of the IR draft (pull request 222: `F2`, `F5`, `to_f2`, `to_f5`, `residue` and the `field_*` operations) | KN3.2 | None names them; the `nu` and `correction_set_check` rows use them (M5) | None: CT-LB1-01's entries are LP3's library code |
| Word operations in the IR draft: concatenation, inverse, conjugate, commutator. Built (pull request 176), with `compose`, `length` and `word_key`, Full's free reduction; menu words (pull request 210), LB1.10's `trace-key/1` for 600-cell-Full words (pull request 214) and its jumbling half, `jumble-locality/1` with `trace_key` on menu words (pull request 251). Not built: these operations on compressed words, and a jumbling word key | LG2.13 to LG2.16 | CT-LG2-11 to CT-LG2-14 | CT-LB1-02's key half; the library entries of CT-LB2-01 and CT-BND-18 are not written yet (CT-LB1-06's jumbling half runs `menu_traces`, C8) |
| Collection types in the IR draft: lists, maps, sets. Built (pull request 176); they cross the runtime's wire (C3, pull request 180) | LG2.20, LG4.3 | None names them; CT-LG4-01 checks host collection layouts | None |
| The IR features CT-RTE-04's other 9 traces need (the two meter differences, `nested-calls`'s frame price and an `emit` row's single charge, were fixed by LP2's M2 meter, PR 170; exact multiplication and pointwise assertions came with LP2's pull request 216, generator draws with pull request 219, child scopes with pull request 225, loops with pull request 234, byte decoding with pull request 249 and streamed words with pull request 250). One waits, `child-decode-error-releases-reservation`, for a child's bytes without a row of their own; three are amendment candidates for C10 (reasons in LP2's pull request 234); form 2's five need `lp1-exec-region-draft-1`'s executor, which LP2 does not build (LP2 plan, section 3, selection 1; stated to the owner on 18 November 2026), so they are not applicable, which is no pass (PHS.63): an item for the owner at C10 | DET.56, DET.57, DET.65 | DET.65's trace vectors in `contract-vectors/vectors.json` | CT-RTE-04 |

Found later, on 16 November 2026. KN5's one implementation, which LP2 writes in `c600-command`, takes two LP3
requirements the LP2 integrator accepted: LP3's KN5.21 lead, closed in its stages 1 and 2 (pull request 174), and the
presented-frame input to `present` that CT-ONE-29's shell-side refusal needs (stage 5, built in pull request 194). Sent
to the LP2 integrator the same day: `c600-check` rules that decide CT-LB3-01's checker half (typed checkpoint and
sticker-test results, LB3.2) and CT-LB3-07's recommendation rule (LB3.5), which LP2 has since built (pull request 213);
a field in `c600-runtime`'s job options, so the runtime's child applies the built-in column itself, not yet confirmed;
and a list of words in the IR, or KN2.10's
streaming, for chunked recipes on the source route, both of which LP2 has since built (pull requests 176 and 170).
LP3 built chunked transport on the source route in the host and the bridge, which carry the solver's own word steps
(C5, section 7). LP2 built the built-in column as `c600_check::admit_built_in` (PR 170); the three built-in tools admit
their entries with it, under class rows that are an LP3 draft until BND.57's rows by class exist as data.

Found with C6, the same day. The representations tool's six rows need command-layer work that KN5's one implementation
now owns: a dispatch from a stored row to a built-in tool's entry, the runtime's view and workspace sinks as a job's
result, argument forms for the rows' reference types, and a workspace revision that moves (`c600_repr::rows`);
`set.save` also needed a list type in the IR draft, which LP2 added (pull request 176). CT-EV3-07's explicit workspace
command joins the durable action order on M6's store (C7), but through the command layer it needs KN5's shared
`session.undo` order (KN5.20, EV4.15), so the test moves from "whole now" to "part now". Sent to the LP2 integrator the
same day. KN5's stages 3 to 5 built that order (pull request 194) and C7 runs it; the test stays part until LP3 wires
`c600-view`'s view model to EV3.2's view rows, which LP2 built in pull request 271 (landed through 272). LP2 has since
built the dispatch (pull request 200), the argument forms (pull requests 202, 256 and 269), the output sinks (pull
request 203), `filter.apply` with its workspace inverse (pull request 256), the structure rows (pull request 262), the
argument blocks (pull request 264), KN5.26's admission matrix L1 and L2 (pull request 270) and a durable home for a
word's part blocks (pull request 272); sections 7 and 8 say what LP3 takes up.

Found with C8, the same day. `c600-jumble` offered no slot action of K⁺, so the slot of a piece at a non-identity
lattice pose was not computed (the answer is `undefined` with its own reason, distinct from `off_lattice`), and it names
no rotation plane for a twist, so a jumbling transform has no swept preview. Both were sent to the LP2 integrator. M8's
levels (pull request 182) added the slot action (`levels::labels`, `Tables::slot_of`), and LP2's kernel pieces (pull
request 190) name the rotation plane (`twist::rotation_plane`). C8 uses both (section 8).

Found with C8's next piece, 16 November 2026. The kernel's label protection (`ProtectedSet`) covers the moving orbits 0
to 34 only, so a centre piece has no label-level protection row, and CT-LB3-02's label verdict is composed from KN2.4's
`positions` rows; M5 protects one letter, so Strict protection of a whole word waited for M8. Sent to the LP2 integrator
the same day. M8's levels (pull request 182) protect any piece of a whole word at pose level, under Net or Strict
(`levels::derive`), and project poses to labels (`levels::project`); the kernel's `ProtectedSet` still covers orbits 0
to 34 only, and LP2's kernel pieces (pull request 190) protect named pieces at label level, centres included
(`Kernel::piece_protection`, `strict_piece_protection`); CT-LB3-02 runs whole on these (C8, section 8). On LP3's side,
`c600-display`'s slot projections bound a state digest but carried no level (C2), so CT-LB4-03's "no lift from labels"
rested on the digest that binds the subject. Fixed the same day: each projection holds one level and answers a subject
at the other level with `KN1:input.level_not_held` before any digest is compared (KN1.8, KN1.10); CT-RPI-12's
off-lattice part and both halves of CT-LB4-03 check it under a planted equal digest.

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
3. **Session store stand-in.** Step 9's durable transaction sits behind a `Store` trait. `MemoryStore` stood in until
   KN5's one implementation (PHS.18): its stages 1 and 2 (LP2, pull request 174) make `c600_session::JournalStore`
   the layer's store over KN4's journal, with `CommandLayer::open` replaying its history. Nothing on `MemoryStore` is
   persistence evidence; C7's tests on M6's store and through the layer over `JournalStore` are.
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
| C8 | The jumbling tool | M5 (on main) | Built in part (section 8) |
| C9 | The on-screen half and the owner-machine acceptance runs | The renderer freeze (signed 16 November 2026, ADR-R01); the owner's runs | Started on 18 November 2026 (owner): its components in a line of their own (section 8) |
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
  and the 600-cell-Full half of CT-RM-07. CT-RPI-08's jumbling rows came with C8.
- C2, views (`rust/c600-view`, branch `claude/lp3-view`), built: CT-ONE-30, CT-RM-02, CT-RPI-03, CT-RPI-06,
  CT-RPI-07, CT-RPI-36 and CT-RPI-37, the limit half of CT-RPI-22 and rows 1, 2, 7 and 9 of CT-RM-09.
- C3 (`rust/c600-runtime`, branch `claude/lp3-runtime-process`), built: the runtime in a process of its own; 8 of
  DET.65's 40 traces of CT-RTE-04 and the headless fault half of CT-RTE-10. C5's host bridge makes it a client of the
  command layer (ADR-L02). Under LP2's M2 meter (PR 170) three more traces run on both executors (`nested-calls`,
  `output-equality`, `output-retained-at-cutoff`): 11 of 40. With LP2's `exact_mul` and `assert` (pull request 216) six
  more run (the three exact-multiplication traces, `alias-release`, `alias-still-live` and `pointwise-failure`): 17 of
  40. With LP2's `rng_init` and `rng_draw` (pull request 219) three generator traces more run, and with its child scopes
  (pull request 225) the five child traces, child records included: 25 of 40. With its loops (pull request 234)
  `final-loop-test` runs: 26 of 40. With its byte decoding (pull request 249) `decode-scalar` and `decode-surrogate` run
  too, on bytes the host delivers as a snapshot: 28 of 40. With its streamed words (pull request 250) the three
  streaming traces run, the split and the coalesced trace as one job, since no host call or answer falls among a
  stream's rows: 31 of 40. Of the other 9, one waits for the IR draft (LP2: a child's bytes without a row
  of their own, for `child-decode-error-releases-reservation`), three are amendment candidates for C10, and form 2's
  five are not applicable, which is no pass, since LP2 builds only form 1 of LG2.5 (section 9). A compressed word
  crosses as its parts, within one frame's bound on the lengths they derive (crate draft 15, on LP2's pull request 185).
- C4 (`rust/c600-module`, branch `claude/lp3-module-closure`), built: module closure, its lock and the approval bound
  to the closure digest; CT-RTE-08's source leg and 36 of LX-3's 75 frozen keys of CT-RTE-07 on 600-cell-Full;
  three more (A08, A17 and A18: a word's length, inverse and concatenation) with LP2's batch 2 (pull request 176),
  39 in all. CT-RTE-08's persisted words come with C7; CT-RTE-07's other keys wait for M5, C7, the IR draft and C5.
  A built-in approval of a program for another IR version now gives the checker's code, as the other author columns
  do (a gap LP2's conformance tests for LG5 found).
- C2, display next (`rust/c600-display`, branch `claude/lp3-c2-display-next`), built: CT-RPI-14 over the drafted chart
  repertoire (RPI.34 is open), CT-RPI-18 on the CPU reference, CT-RPI-19's Windows leg, CT-RPI-22's chart half (so
  CT-RPI-22 runs whole) and the headless part of CT-RPI-32. The rest of CT-RPI-32 needs a backend (C9).
- C2, views next (`rust/c600-view`, branch `claude/lp3-c2-view-next`), built: CT-RPI-23, whose two parts both run,
  the counter half of CT-RPI-26 and the admission semantics of CT-EV3-08. Engine epochs now bind revisions, bundles
  and output, and a scheduled recomputation can neither open nor close a view.
- C2, stage take-up (`rust/c600-display`, branch `claude/lp3-c2-stage-checker`), built: the display takes up the stages
  that LP2's stage checker admits (pull request 241: `admit_stage`, RPI.68 to RPI.71). CT-RPI-32's headless part reads
  its stage descriptions from admitted stages, and each refusal of the checker comes back with the checker's code; the
  display's own rule on a stage description keeps its tests. The headless parts of CT-RM-09's rows 3 to 6, 8 and 10 run:
  RPI.72's raster bound and allowance on the CPU, the frame stop, the session's quarantine with RPI.77's fallback, and
  RM.41's frame reports, under declared fixture weights in place of RM.38's measured table (RPI.79). The rest waits for
  C9: the vertex evaluation, the projection and clip classification, U on a device, the skipping of a stopped frame's
  work, and the injection and detection of a device fault (CT-RM-10, CT-RPI-31, CT-RPI-34). So do CT-RPI-26's gate half
  and CT-EV3-08's measured half. Two tests in `c600-tools` and `c600-repr` still build stage descriptions as fixtures;
  they can read admitted stages with those crates' next pieces.
- C2, source edits (`rust/c600-view`, branch `claude/lp3-view-source-stale`), built for C9's LX-4 (its question U8,
  19 November 2026): `ViewModel::edit_source` takes the editor's draft of a module version, and output that version
  produced shows its view's stale state from that call on, with the producing version and the draft in its status and
  its frame. Nothing runs or is fenced (EV1.7); the output is still drawn while its base matches, and its picks still
  open the producing version (EV3.5, EV3.6). This is the integrator's reading of EV3.6 and RTE.49's stale-state
  interval: ONE.12's table names the view-stale condition for a lagging revision only, and a lagging source is a second
  trigger of the same D2 state, which shows beside the first and never stands in for it. No frozen text changes. A view
  runs the checked draft once it is reopened on it after activation (EV1.11 and CT-EV1-08 are LP4's). LX-4 times the
  state on the owner's machine (C9); nothing here is timed.

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
   with the tool's views, CT-EV4-02, CT-EV4-04 (the append, preview, commit and twist legs) and CT-ONE-29 that the
   tool and the bridge make reachable, each naming its part.
3. A change to `c600-command` gets C1's fresh read; C1's item 5 holds.

- C5 (`rust/c600-tools`, branch `claude/lp3-source-route`), built: item 1 (compare.py equality on all 3,201 steps,
  traces byte-identical to the command route's, and receipts that agree in all 13,553 pairs apart from their
  envelopes); item 3 (C5 changed no file in `c600-command`); and of item 2, the 600-cell-Full tool parts of CT-LB6-03
  and CT-BND-15, the headless part of CT-ONE-29 and the command-layer parts of CT-BND-07 and CT-BND-11. The tool runs
  in process.
- C5, input router (`rust/c600-input`, branch `claude/lp3-input-routing`), built: of item 2, CT-EV4-02 and CT-EV4-04's
  append, preview, commit and twist legs and its busy leg, on synthetic events through the shell's public API; ahead of
  LP4, CT-EV4-06's synthetic half. The twist keys type exact requests of the twist menu the host pins (A6, KN3.7) and
  reach `twist.apply` (pull request 207); which key types which request is a draft that settles nothing in the chapters.
  With the router joined to the tool's view model and views, CT-BND-19's live-twist part runs: until the twist key's
  key-down commits the twist, no field of the view model holds a value of any twist a key could release at the fixture's
  four grip slots, since the view model's only input is the layer's bundle and the layer has not moved.
- C5 next, the process host (`rust/c600-tools`, branch `claude/lp3-c5-process`), built: the tool on C3's runtime process
  (`c600-runtime`'s child), with the same boundary and bridge in the caller. `c600-lp3-source --host process` replays
  O01 to O07 with traces byte-identical to the in-process host's, and every record names its host; four O07 steps whose
  words have a letter outside 1 to 1200 in magnitude cannot cross the runtime's wire, so they have no receipt on the
  process host and their records keep the kernel's refusal. Of item 2: CT-LB2-02's Net half; CT-LB2-03's oversize
  refusal on both routes, with chunking on the command route; CT-LB3-07's checker half for outcome tables and named-word
  checks. CT-LB3-01's checker half and CT-LB3-07's recommendation rule are `c600-check`'s, built by LP2 (pull request
  213). A test
  confirms a lead on KN5.21 to KN5.23 (a refused forgery takes a genuine command ID's retry record), sent to LP2's KN5
  implementation.
- C5 next, the tool's views (`rust/c600-tools`, branch `claude/lp3-c5-views`), built: CT-EV3-01, CT-EV3-02 and
  CT-EV3-04, whole for the tool's view; CT-EV3-03, whole, with its imported half on a fixture import record, since the
  Rust layers have no import row; CT-BND-19's part for the tool's views; CT-ONE-29's user-stage part at the host's
  adapter, whole since LP2's stage 5 (pull request 194) gave the layer the presented-frame input to `present` that the
  shell's own refusal of a user-stage frame needs; CT-BND-19's live-twist part runs in `c600-input` on this view model
  and these views (the input router's item above).
- C5, the host's provenance reader (`rust/c600-tools`, `workspace`), built: the reader KN5's stage 3 asks of the host
  (PS1.16, KN5.26, BND.84, BND.85), which reads a record's block and admits a record draft with the boundary's preview
  admission over the blocks the host keeps and the part blocks the session keeps. It lists the parts of a record's
  block that the host keeps, recursively, which the layer keeps with the record's write (CT-BND-27; LP2's pull request
  272). `c600-lp3-source` installs it, and C7's tests on the layer run with it.
- C5, chunked transport on the source route (`rust/c600-tools`, `bridge`), built: of item 2, CT-LB2-03's chunking part
  on the source route (LB2.3, LB2.4, LB1.11). For one run the host gives the bridge the solver's word steps of a recipe
  of plain word steps only; a proposal of exactly that recipe's literal word longer than one chunk (100,000 letters)
  goes in those steps in its one invocation, and every other proposal in one word step, so O01 to O07's invocations do
  not change. Two full chunks preview on both hosts with the command route's digests and end state and commit as one
  transaction; a word that is not the input literal is refused whole. A recipe with a star step, or with a word step
  that has another field, keeps one word step.
- C5, library (`rust/c600-library`), built: LB1's literal words as language code
  (LB1.1, LB1.8). The module `words` (`words.6cs`) exports a pure `power`; concatenation, inverse, conjugate and
  commutator are the IR draft's own operations (LG2.15); the row's catalogue entry is Rust data. Of the tests:
  CT-LB1-03's 600-cell-Full half (construction reads no state, application checks each prefix through
  `Kernel::replay`, and an inverse returns an admitted path to its start) and CT-LB2-02's Strict half at the kernel, on
  library-built words.
- C5, Strict protection through the tool's route (`rust/c600-tools`), built: of item 2, CT-LB2-02's Strict half on both
  hosts, on the command layer's Strict mode (LP2's pull request 198). After the shell's `protection.policy.set` to
  Strict, the tool's identity word with a moving first letter previews with the kernel's Strict row and the command
  route's summary, and its commit is refused with `ORBIT_WOULD_MOVE`; with no orbit protected it commits. The runtime
  cannot set the policy through the bridge (`LG3:capability.not_granted`). The captured lock and LB2.3's coverage report
  are library rows over kernel rows and wait for host rows for the rows they read; the tool's bridge answers a snapshot
  read with an argument for `positions` (LP2's pull request 226) and `labels` (C5 library) only.
- C5, the menu-word gate (`rust/c600-module`, `run`), built: a menu word's proposal (LB1.8; LP2's pull request 210)
  passes the same capability-separation gate as a literal word's, so a proposal whose block may read state is refused at
  the preview point with `LG3:capability.not_granted` (LG3.13, BND.39), and the canonical result keeps a menu-word
  proposal's letters and alphabet (a CT-RTE-08 case in `ct_rte_08.rs`). The tool's jobs pin no menu and no trace
  relation yet (`JobOptions::menu`, `JobOptions::relation`): no built-in entry takes a menu word or computes a trace
  key, and the first that does wires them.
- C5, library, exact completion (`rust/c600-library`, `completion.6cs`; the `labels` row in `rust/c600-tools`), built:
  CT-LB2-04, whole, on 600-cell-Full. LB2's exact completion is language code,
  `completion.exact_completion(pieces, home_slots)`: it reads KN2.4's `positions` and `labels` rows with `read_snapshot`
  and answers `true` only for a non-empty list of named pieces whose stickers all sit in their own home slots. A label
  is the home slot of the sticker a slot holds, so the entry reads identities, slots and orientations, never colours.
  The tool's bridge serves the `labels` row beside `positions` as a draft (its argument, answer, codes and tariff are in
  the crate documentation), and the built-in column admits the entry with both read rows, so it reads state and prepares
  no preview. The whole run (`c600-tools/tests/exact_completion.rs`) goes through `Boundary` and the bridge onto the
  command layer, built in, in the solving profile: the solved state passes, and an empty list, a piece turned in place,
  a colour-preserving exchange of two pieces and a permitted residual at a buffer do not.
- Next, in C5's crates: the other 600-cell-Full library entries as language code on what the IR draft now has (F2 and
  F5, loops, `trace-key/1`), with a host row in the tool's bridge for each kernel row an entry reads; the jumbling
  halves, the menu-word library entries among them, are C8's (section 8), and the other parts above wait for LP2.
- C5's host, runs inside a solver command (`rust/c600-tools`, `process`), built: LP2's pull request 247 built the
  layer's check that a program job's run lies inside a solver command (B2; the owner's reading 2 of 18 November 2026)
  and left it off (`SOLVER_COMMAND_CHECK`). LP3's hosts now start every run through the tool's row
  `tool.full_600_cell.preview_word`, a query overlay row the shell invokes with the word: the host takes the program
  job, runs the tool as the job's run `job-<id>`, and reports the job with the program's acknowledgement once the run's
  preview is built (a run refused at its start stage fails the job with the refusal's code). The source route, both
  hosts and every `c600-tools` test run this way, and the source-route check counts the solver's command and its
  report in each step's receipts. Next: the KN5 session's small pull request switches the check on and runs the B2
  attack test that is ignored until then.

CT-LB1-01, CT-LB1-03, CT-LB1-06, CT-LB2-01, CT-LB2-04 and CT-BND-18 test library entries, which are language code
(LB1.1). The IR draft `c600-ir-draft-1` has the word operations (LG2.15), compressed words (LG2.16) and collection types
(LG2.20, LG4.3) since LP2's pull request 176, menu words (LB1.8) since pull request 210, LB1.10's `trace_key` since pull
request 214, the F2 and F5 number kinds since pull request 222 and loops since pull request 234, and C5's library writes
powers and exact completion. Entries over kernel rows need a host row for each kernel row they read (LP2's pull request
199 added the IR draft's snapshot read with an argument; the tool's bridge answers it for `positions` since LP2's pull
request 226 and for `labels` since C5's exact completion); CT-LB2-04's exact completion is such an entry, and so are
CT-LB2-01's recipes and reports, whose host rows LP3 adds. The other library entries these tests need are not written
yet (section 2).

C6, the representations tool, headless half (selection 7). Acceptance: CT-LB4-02, the 600-cell-Full half of CT-LB4-03,
the admission and CPU half of CT-LB4-04, the CPU reference of CT-LB4-01, the counter half of CT-EV3-06, the
view-cannot-write half of CT-EV3-07 and the representations parts of CT-LB6-03 and CT-BND-15, each naming its part;
its rows meet C5's item 1 through the source and command routes; C1's item 5 holds.

- C6 (`rust/c600-repr`, branch `claude/lp3-c6-repr`), built: CT-LB4-02, whole (synthetic); the 600-cell-Full half of
  CT-LB4-03; the admission and CPU half of CT-LB4-04; CT-LB4-01, whole over `c600-display`'s drafted chart repertoire
  (RPI.34 [O]: a declared domain decided by an exact region test and section membership on the posed vertices,
  boundary cases included, a chart whose vertices all pass and whose region check fails inside refused, and
  injectivity, embedding, isometry and homomorphism each decided by its own check on `c600_jumble::polytope`, with the
  stereographic chart's isometry and homomorphism left `not_established`); the counter half of CT-EV3-06; the
  view-cannot-write half of CT-EV3-07 at the command layer; and the representations parts of CT-LB6-03 and CT-BND-15.
  The tool is a `.6cs` module with six entries admitted under the built-in column, with a Rust manifest of the class
  `view_aid`; the class review covers every exported function and only the source bytes the manifest names. C6 changed
  no file in `c600-command` (C1's item 5).
- C6's rows (`rust/c600-repr`, `tests/served_rows.rs`), built: five of the six rows meet C5's item 1 through the
  source and command routes, on LP2's served rows (pull requests 200, 202 and 203). A test host serves
  `filter.preview`, `view.effect.scope`, `set.save`, `structure.compare` and `structure.layer` with the entries
  `c600_repr::rows` names and runs each job's entry in process through `c600-run`'s boundary. Invoked from the shell,
  the two queries and the view complete transiently at the job's revision; `set.save` and `structure.layer` each
  complete as one durable workspace action with the output's digest as the result reference and no undo entry. The
  source route, the entry's own run in process through `c600-run`'s boundary with no layer (not C5's runner or the
  runtime's own process), gives the same value, views and records for the same arguments and state; a stand-in engine
  answers the two snapshot reads with the state number on both routes. Only the command route has the receipt, the
  layer's form and revision checks and, for a workspace row, the durable action.
  `c600_repr::rows` records what the layer meets and what remains for each row. C6 changed no file in
  `c600-command`; its tests take `c600-run` as a dev-dependency.
- C6, next. LP2 has built what each item needs, and the crate documentation names the APIs. `filter.apply`: the layer
  serves the row since LP2's pull request 256, with the gate `fresh_filter_preview`, the `filter` preview rule, the
  `FilterRef` and `FilterPreviewRef` forms and a workspace inverse. LP3's test only serves it and checks the refusal of
  a stale filter preview, so the row does not meet C5's item 1 yet. Next: the entry's parameter, a natural now, becomes
  the layer's `FilterPreviewRef`, and the row runs through both routes. Argument blocks: LP2's `invoke_with_inputs`
  (pull request 264) lets a host give a running program's job the provenance blocks of its arguments. The test host
  builds none, so it still fails such a query or view job (since LP2's pull request 237 the layer refuses a workspace
  row such as `set.save` from a running program with `LG3:capability.not_granted`, the owner's narrow reading of ONE.52
  of 18 November 2026). Next: the host builds the blocks from the run's context. Kernel rows: LP2's structure rows (pull
  request 262: `compare_cells`, `cell_layers` and `layer_counts` on the layer, which read no state and refuse with
  Jumble on) answer the `structure.compare` and `filter.preview` snapshot reads. Next: the entries pass their arguments
  to a snapshot read, and the tool's bridge serves the rows, in place of the test host's state number. LP2's pull
  request 269 checks the `CanonicalObjectRefs` form: a member that is no piece or position (0 to 177,119) is refused at
  admission, and an empty list and repeats pass; LP3 keeps that form. Still not built: a record and an exact-set digest
  for each saved set, and a display that applies the published scope. Also not built: every device step (C9; the stage
  checker is LP2's since pull request 241), the relationship strip (C8 computes one of its typed relations, a slot's
  current occupant, at pose level), and a BM5 review record, which the tool gets when it ships.

Stop condition for both: each ends when its items pass in CI and its pull request is merged. Each stops earlier under
C1's conditions, or when a tool needs a kernel row LP2 has not built.

## 8. C7 to C10: what waits

- **C7, durable receipts on M6's store.** KN5 has one implementation, owned by LP2 under PHS.18 (integrator, 16 November
  2026), in `c600-command`, with `c600-session` implementing its `Store` trait; its stages 1 to 5 landed in pull
  requests 174 and 194, and LP3 no longer holds back from `c600-command` and `c600-session`. Built (`rust/c600-durable`,
  test-only), on the command layer over `JournalStore` with C5's provenance reader installed: CT-RTE-08's persisted-word
  case, its durable leg (the record and its receipt come back equal after a power loss and in a new session; the reader
  refuses a solver tool's record draft at preview; a built-in tool's record draft goes stale when its record is written
  again, and that write keeps the word's part, with which a host that keeps no block admits the draft after another
  power loss: CT-BND-27, on LP2's durable home for part blocks, pull request 272, which C5's reader serves with
  `parts` and `admit_kept`); CT-EV3-07's durable action order in the shared `session.undo` order, and its view part on
  a language overlay row standing in for a view. CT-LB2-03's named work-sheet reuse through the layer's
  `worksheet.save`, `worksheet.load` and `worksheet.compare` (LB7.2, LB2; LP2's pull requests 232 and 277;
  `tests/ct_lb2_03_work_sheet_reuse.rs`): a saved sheet comes back equal at a reuse in the same session, after a power
  loss and in a new session; each reuse makes the presented preview stale; a comparison analyses the sheet afresh with
  LB2's gains and losses and reports its macro version, writes nothing and keeps a presented preview fresh, and comes
  back equal after a power loss; and each refusal of the three rows that LP2's draft names comes back with the layer's
  code and changes nothing, the journal's bytes included. The refusals with Jumble on run in LP2's `c600-command`
  tests, not here. On M6's public API: the same CT-EV3-07 parts and CT-LB2-03's work-sheet part. LP2 has since run
  KN5.26's admission matrix L1 and L2 through the layer (pull request 270) and built EV3.2's view rows (pull request
  271, landed through 272: `view.open`, `view.close` and the `Views` trait). Next (LP3): `c600-view`'s view model
  serves `Views` for CT-EV3-07. Waits for LP2: a session profile (BND.14, BND.15), and reading a position role's
  occupant again at a work-sheet reuse. M6's session lock (LP2, pull request 195) covers `JournalStore` too. Since LP2's
  pull request 242 every disk takes it, a memory disk included (`MemDisk` is behind the crate feature `mem-disk`, which
  `c600-durable` enables), and a reopen checks each stored commit and release again (finding B7, the owner's reading of
  18 November 2026): a commit not made by the shell, a commit that moves a protected orbit or a release without its
  stored confirmation leaves the session in read-only recovery with `KN5:consistency.unadmitted_journal_action` (a draft
  code). Next: CT-RTE-07's six store keys, checked again against M6's run record (LP2's pull request 235).
- **C8, the jumbling tool.** M5 is on main (pull request 167). Built (`rust/c600-jumble-tool`, and `jumble` in
  `c600-display`): the tool's `.6cs` source with two entries admitted under the built-in column and a Rust manifest of
  the class `checking`; the jumbling parts of CT-LB6-03 and CT-BND-15; the kernel halves of CT-LB1-02, CT-LB3-03,
  CT-LB3-06 and CT-LB3-07 on M5's public API; the producers of the jumbling rows J02 to J05 with CT-RPI-08's jumbling
  rows; CT-RPI-12's off-lattice part, so CT-RPI-12 runs whole with C2's stale half; and CT-RM-07's jumbling half on M5's
  model, rows 1 to 8 and 15. Taken up since (pull requests 278 and 288): Jumble on in the command layer (LP2's pull
  requests 212, 217, 218 and 220), with oracle equality against M5's W-J fixtures through `jumble.on` and
  `jumble.twist.menu` (`tests/jumble_on_layer.rs`: the first 130 records of each journal by default, the whole journals
  on request), and CT-LB4-03's layout evaluated again at a later revision of a jumbled session, pressed and undone
  through the same layer; and CT-LB1-06's jumbling half on `jumble-locality/1` (LP2's pull request 251), which
  `c600-jumble-tool` pins (`locality`), with the library entry `menu_traces` (`tests/ct_lb1_06.rs`). Not taken up yet
  from LP2: through the command layer, `jumble.off`, the refusals, redo, protected pieces (since pull request 254
  protection holds inside Jumble, so the layer refuses a pose move that moves a protected piece) and free mode, which
  LP2's route test and `c600-command`'s own tests run. Waits: the rows J01 and J06 to J12 (their producers in
  `c600-display`; J10 also needs exposed-face geometry, LP2); CT-LB1-02's key half (LG2.18's jumbling word key, LP2);
  BM5's review records. Built next, as one piece: CT-LB1-03's jumbling kernel half (LB1.8's literal words of one pinned
  menu and their constructors, with no state access); CT-LB3-02's LP3 side (protection verdicts that carry their level
  and are refused at the other, beside the kernel facts M5's `crossing.rs` runs as CT-KN2-03); CT-EV3-06's off-lattice
  half (a slot's current occupant, with "none: off the lattice" and `unknown` as two values, over all 259,800 slots);
  and CT-LB4-03's jumbling half (J02's posed row through C6's layouts, at pose level). M8 has since built what that
  piece lacked (pull requests 176 and 182): the slot action of K⁺ (`levels::labels`, `Tables::slot_of`); LB3.4's
  projection from poses to labels with its checkpoints (`levels::project`, `retained_checkpoint`, `label_checkpoint`);
  the sticker test; a whole word's Net and Strict protection at pose level (`levels::derive`); and the repair list,
  covers and release check (`observe::cuts`). C8 now uses the slot action, the end tests and the repair list: a slot's
  current occupant at every lattice pose (CT-EV3-06); and the kernel halves of CT-LB3-01, of CT-LB3-03's sticker test,
  and of CT-LB3-05 with CT-BND-17. With LP2's kernel pieces for C8 (pull request 190), `c600-display`'s pose projection
  places a sticker at any lattice pose by the slot action with its frame check (`Tables::piece_slots`; J13), so
  CT-LB4-03 checks the display's own projection against the current-occupant relation; a jumbling transform carries the
  plane the engine names (`Twist::plane`), so it has a swept preview, a half-turn at θ = π in the engine's direction
  (J03); and CT-LB3-02 runs whole on engine verdicts: M8's protection of a whole word at pose level, the kernel's
  protection of named pieces at label level, centres included (`Kernel::piece_protection`, `strict_piece_protection`),
  and M8's projection from poses to labels at the crossing. With LP2's ν and correction catalogue (pull requests 201 and
  206), CT-LB3-04 runs whole at C8's level: off-lattice ν is undefined, the pinned C5 convention round-trips, a planted
  nonzero fixed I_a coordinate is refused as a consistency error, and a drafted correction set is checked as drafted,
  the tool choosing none. CT-LB3-01's kernel half takes its ν case (a reachable nonzero-ν lattice state fails the label
  checkpoint, a ν-zero one passes), and CT-LB3-07's correction part runs on the kernel. The jumbling library entries are
  built as 600 Cell Script on the IR draft's menu words (LP2's pull request 210), in `c600_jumble_tool::library`:
  LB1.8's literal words of one pinned menu (`power`, pure), LB1.10's trace keys of menu words (`menu_traces`, pure),
  LB3.1's "ν and drafted correction-set check" (`jumble_nu`, on F2 and F5, pull request 295), and LB3.1's rows "Heights,
  branches, domains and Φ", "Release and whole-word checks" (the chronological observations and the step flag) and
  "Protection" at pose level (a whole word's Net and Strict verdicts, as M8's `levels::derive` gives them). Each entry
  that reads state reads only its KN2.4 rows. No host bridge or command-layer route serves those rows to a program yet,
  so the tests serve them from `c600_jumble` through a test host, and the rows' answer types are drafts; that route is
  next. The jumbling halves of CT-LB1-03, CT-LB3-03 and CT-LB3-06 now also run in the language, and all three still run
  in part. `jumble_nu` reads ν and checks a drafted set in F2 and F5, through the tests' host (`tests/ct_lb3_04.rs`).
  Not taken up yet: the rim and survey rows of CT-LB3-07. F03 on 600-cell-Full has no swept preview: the retained kernel
  names no plane for its twists, and which engine names one is not settled.
- **C9, the on-screen half, on the selected renderer.** The owner signed the renderer freeze on 16 November 2026
  ([freeze record](../../wiki/decisions/renderer-freeze.md), ADR-R01): S-A2, the S-B Direct3D 12 drawing core as a
  shared native library hosted by Godot 4.7.2 .NET. The owner started C9 on 18 November 2026 as a line of its own, with
  no Godot source build. The same day the owner accepted route B for the confirmed present, for now
  ([decision](../../wiki/decisions/owner-decisions-2026-11-18-route-b.md)): the core presents in its own composition
  swap chain, on its own Direct3D 12 device, in a DirectComposition layer beneath Godot's transparent window. Three
  owner-machine checks make it final: input through the transparent window, the skew between the UI frame and the 3D
  frame, and three cold runs of the probe. They are batched with the runs below, and a pass needs a revision of
  ADR-R01.2 under PHS.52. DXC ships as the run-time compiler for user stages (RPI.80, ADR-R01.4). That line builds the
  components: the core with W-J's pose path, a bridge crate that implements `c600_display::Backend` over the core's C
  interface, the product shell and the measurement tools; LP3 keeps the test mapping and `c600-display`, `c600-input`,
  `c600-runtime` and `c600-view`. On main so far: the core in `renderer/core/` with W-J's pose path as ABI 3 (pull
  request 240) and the product API as ABI 4 (pull request 252: one C call per RM.14 operation, where `sa2_presented` and
  `sa2_recover` answer unavailable), whose W-J runs, three cold W3 runs and ABI 4 GPU paths are still unrun; the harness
  for those runs (`renderer/acceptance/`, pull request 248); DXC v1.9.2609, pinned in the toolchain lockfile (pull
  request 244); a present probe through Godot's public API (`work/experiments/c9-present-probe/`, pull request 245),
  experiment code whose runs give probe evidence for a PHS.52 design proposal only; the display host's bridge (pull
  request 259: `c600-render` implements `c600_display::Backend` over ABI 4 through a safe call interface, with a fake
  core for headless tests, and `renderer/bridge` holds the C function table; J01 and J02 are reported unsupported, and
  nothing has run against the built core library or a GPU); and headless tools for charter E2 and E3 (pull request 261,
  `tools/perf/c9`, with draft log formats; they read logs and captures, so a measurement needs the owner's machine).
  Since then: route B in the core as ABI 5 and the bridge on it (pull request 263: a context on the core's own device
  with a composition surface; `sa2_presented` confirms a frame by the probe's exact rule; `sa2_recover` rebuilds the
  device as a transaction, and the bridge forgets the old epoch's resource sets); the product shell's steps S1 to S5
  (pull request 267, draft: `c600-shell-host` puts `c600-display`'s host behind a C interface, and its `m6h_recover`
  publishes the view again, which `Host::recover` does not do yet: RM.23, LP3's); and LX-4's module set and edit script
  with LX-2's metered, forced-kill and allocator-ceiling trials (pull request 268: `rust/c600-lx4` and `rust/c600-lx2`,
  measurement tools whose runs need the owner's machine; the `lx2-faults` hook in `c600-runtime` that the fault trials
  need is LP3's and came after it). The tests: the windows tests CT-ONE-26, CT-RPI-13, CT-RPI-21, CT-RPI-24, CT-RPI-31,
  CT-RPI-33, CT-RPI-34, CT-RM-05, CT-RM-06, CT-RM-10, CT-BND-20 and CT-RTE-10; the performance tests CT-KN7-06,
  CT-LG4-09, CT-EV1-06, CT-ONE-25, CT-RPI-26, CT-RTE-11 and CT-RTE-12; the renderer halves of CT-LB4-04 and CT-EV3-08.
  The runs follow PHS.64 on one frozen candidate, with their bars unchanged, and are done together on the owner's
  machine (owner, 16 November 2026), with W-J's runs (ADR-R01.3, option A: a failing run reopens the selection),
  RPI.79's gate run on the product build and route B's three checks; LX-4's time box is 4 hours, on the same day (owner,
  18 November 2026).
- **C10, the exit.** H-08 freezes (PHS.53), with the draft codes registered under PHS.52; every acceptance item has
  evidence on one frozen candidate; the 82 tests run whole; the owner signs off.

## 9. Section 15.4's 82 tests

"Now" means the whole test runs now, with no need of the owner's machine, the on-screen half (C9) or an item LP2 has not
built. 37 tests run whole now (CT-RPI-19 also needs a Linux run for its record), 28 have a part that runs now, and 17
have no part that runs yet. The milestone is in parentheses; "next" marks LP3 work that needs nothing more from LP2 and
is not built yet.

| Test | Class | Whole run | Part now |
| --- | --- | --- | --- |
| CT-KN7-06 | performance | Owner runs: charter E2 under a running program job (C9) | - |
| CT-LG4-09 | performance | Owner runs: LX-2 (C9) | - |
| CT-LB1-01 | source | Its library entries, not written yet: F2 and F5 equation ranks and solutions (on the IR draft's F2 and F5, LP2's pull request 222), exact Q(√5) signs, quaternion factor reconstruction and tree outputs, against the pinned research references (C5 library) | - |
| CT-LB1-02 | source | LG2.18's domain-preserving jumbling word key and "keys preserve the end digest", not built (LP2; the owner's answer of 17 November 2026 gave 600-cell-Full words `trace-key/1`); the library entry, not written yet (collection types are in the IR draft, pull request 176) | The kernel half at a blocked cap, with prefix protection reading literal letters (C8) |
| CT-LB1-03 | source | Next: the jumbling half's rows for application through a host bridge or a command-layer route, in place of the test host that serves them from `c600_jumble` (C8) | The jumbling kernel half: literal words of one pinned menu and their constructors (C8); the jumbling half as language code: `power` and the IR draft's constructors on menu words, with no host, and application prefix by prefix and the inverse's path on the test host's rows (C8); the 600-cell-Full half as language code: powers in the library, the IR draft's word operations, application prefix by prefix and the inverse's path (C5 library) |
| CT-LB1-06 | source | "Word keys remain separate" on the jumbling side, on LG2.18's jumbling word key, not built (LP2) | The 600-cell-Full half on `trace-key/1` (owner, 17 November 2026): in source, at run time and on the pinned model's relation (LP2's pull request 214: `c600-syntax/tests/conformance_lg2.rs`, `c600-run/tests/trace_keys.rs`, `c600-tools/tests/supports.rs`); the jumbling relation on the pinned model (LP2's pull request 251: `c600-jumble/tests/locality.rs`); the jumbling half on `jumble-locality/1`, which `c600-jumble-tool` pins, with its keys on M5's states and the library entry `menu_traces` (C8: `c600-jumble-tool/tests/ct_lb1_06.rs`) |
| CT-LB2-01 | source | The library's recipes and reports, entries over kernel rows: a host row in the tool's bridge for each kernel row they read (LP3; the bridge answers `positions` since LP2's pull request 226 and `labels` since C5's exact completion) | - |
| CT-LB2-02 | source | The captured lock of LB2's "Net and Strict protection" and LB2.3's coverage report, library entries over kernel rows (section 7) | The Net half (C5); the Strict half through the tool's route (C5); the Strict half at the kernel, on library-built words (C5 library) |
| CT-LB2-03 | source | Chunked transport on the source route for a recipe with a star step, or a word step with another field, which C5 records as not built (C5) | The oversize refusal and the source route's chunked transport (C5); named work-sheet reuse through the layer's `worksheet.save` and `worksheet.load`, and LB7.2's fresh exact analysis of a reused sheet through `worksheet.compare` with LB2's gains and losses and macro versions (LP2's pull request 277), on M6's journal; reuse on M6's own API (C7) |
| CT-LB2-04 | source | Now (C5 library: LB2's exact-completion entry, LP3's library code over KN2.4's `positions` and `labels`, run through the tool's bridge, which serves `positions` since LP2's pull request 226 and `labels` since this entry; `c600-tools/tests/exact_completion.rs`) | - |
| CT-LB3-01 | source | Now (C8's kernel half on M8's levels, the ν case included, with ν from LP2's `nu` row; the checker half, the end tests' typed verdicts, is LP2's, pull request 213) | - |
| CT-LB3-02 | source | Now (C8; M5's `rust/c600-jumble/tests/crossing.rs` runs the kernel facts as CT-KN2-03) | - |
| CT-LB3-03 | source | Next: the library entry's rows through a host bridge or a command-layer route, in place of the test host that serves them from `c600_jumble` (C8) | The kernel half: heights (0,0) off the lattice in free mode, domain IDs by exact coset tests, and the sticker-test sentence on M8's `sticker_test` (C8); the library entry, equal to the kernel half on six menu-reached states (C8); the agreement with `trees.py` runs in M5's `tests/test_jumbling_rust_candidate.py` |
| CT-LB3-04 | source | Now (C8, on LP2's `nu` and `correction_set_check`, pull requests 201 and 206; the entry also runs as language code, C8's `jumble_nu` on the IR draft's F2 and F5 (pull request 222), through a test host) | - |
| CT-LB3-05 | source | A job that names its cap through the command layer (KN5, LP2); the repair-list inspector library entry, not written yet. Built: the named-cap repair list (pull request 176) and the built-in column's repair-list exception (pull request 229) | The kernel half on `observe::cuts::repair_list` (C8) |
| CT-LB3-06 | source | Next: the library entries' rows through a host bridge or a command-layer route, in place of the test host that serves them from `c600_jumble` (C8) | The kernel half on exact word fixtures (C8); the library entries: peak, excursion, tameness and the step flag recomputed from the rows, and a whole word's Net and Strict protection at pose level, equal to the kernel's and to `levels::derive`'s (C8) |
| CT-LB3-07 | source | The inspector library entries, not written yet. Built: LB3.5's recommendation rule in `c600-check` (pull request 213), the rim summary, covers and release check (pull request 176) and the correction catalogue (pull request 206) | The checker half for outcome tables and named-word checks (C5) and for recommendations and reads (LP2, pull request 213); the jumbling half on the kernel, its correction part on LP2's `nu` and `correction_set_check` (C8) |
| CT-LB4-01 | source | Now (C6) | - |
| CT-LB4-02 | synthetic | Now (C6) | - |
| CT-LB4-03 | source | Now (C6's 600-cell-Full half; C8's jumbling half at pose level, at lattice poses other than the identity through `c600-display`'s pose projection on M8's slot action of K⁺, and at a later revision of a jumbled session, pressed and undone with Jumble on in the command layer) | - |
| CT-LB4-04 | source | The on-screen half (C9) | Admission and the CPU half (C6) |
| CT-LB6-03 | source | BM5's review and its record, when the tools ship; LX-3's cases that need word operations, which the IR draft has since pull request 176 (C4, next) | The 600-cell-Full, representations and jumbling tools (C5, C6, C8) |
| CT-EV1-06 | performance | Owner runs: LX-4 (C9) | - |
| CT-EV3-01 | synthetic | Now (C5) | - |
| CT-EV3-02 | synthetic | Now (C5) | - |
| CT-EV3-03 | source | Now (C5) | - |
| CT-EV3-04 | synthetic | Now (C5) | - |
| CT-EV3-06 | source | The strip's data contract (chapter 06, RPI.65) | Unchanged frames compute no relation (C6); "none: off the lattice" distinct from `unknown`, over all 259,800 slots, with the occupant at every lattice pose on M8's slot action (C8) |
| CT-EV3-07 | source | `c600-view`'s view model behind LP2's view rows (pull request 271, landed through 272; C7, next) | A view cannot write, at the command layer (C6) and on a language overlay row standing in for a view (C7); the durable action order, in the shared `session.undo` order through the command layer over `JournalStore` and on M6's store (C7) |
| CT-EV3-08 | source | RM.39's measured ceilings on the selected renderer (C9) | The admission semantics (C2) |
| CT-EV4-02 | synthetic | Now, on the draft keymap (C5) | - |
| CT-EV4-04 | synthetic | Now, on the draft keymap (C5; the twist leg through `twist.apply`, LP2's pull request 207) | - |
| CT-ONE-25 | performance | Owner runs: charter E2 (C9) | - |
| CT-ONE-26 | windows | The on-screen half (C9) | The token binds the presented preview (C1) |
| CT-ONE-29 | source | Now (C5, through the layer's `present`, LP2's pull request 194) | - |
| CT-ONE-30 | source | Now (C2) | - |
| CT-RPI-02 | source | Now (C2) | - |
| CT-RPI-03 | synthetic | Now (C2) | - |
| CT-RPI-04 | source | Now (C2) | - |
| CT-RPI-05 | source | Now (C2) | - |
| CT-RPI-06 | source | Now (C2) | - |
| CT-RPI-07 | synthetic | Now (C2) | - |
| CT-RPI-08 | source | The rows J01 and J06 to J12: their producers (C8, on M8's observations and levels, pull requests 176 and 182, and ν, pull request 201; J10 also needs exposed-face geometry, LP2); a backend's own refusals (C9) | The 600-cell-Full rows (C2); the jumbling rows J02 to J05 on M5's states (C8) |
| CT-RPI-09 | source | Now (C2) | - |
| CT-RPI-10 | synthetic | Now (C2) | - |
| CT-RPI-11 | synthetic | Now (C2) | - |
| CT-RPI-12 | source | Now (C2, C8) | - |
| CT-RPI-13 | windows | The on-screen half (C9) | - |
| CT-RPI-14 | synthetic | Now (C2) | - |
| CT-RPI-18 | synthetic | Now (C2) | - |
| CT-RPI-19 | source | Now, with a Linux run (C2) | - |
| CT-RPI-21 | windows | The on-screen half (C9) | - |
| CT-RPI-22 | source | Now (C2) | - |
| CT-RPI-23 | source | Now (C2) | - |
| CT-RPI-24 | windows | The on-screen half (C9) | - |
| CT-RPI-26 | performance | Owner runs: charter E2 and the renderer gate (C9) | The counter half (C2) |
| CT-RPI-27 | source | Now (C2) | - |
| CT-RPI-31 | windows | The on-screen half (C9) | - |
| CT-RPI-32 | source | A backend (C9) | The headless part, on stages LP2's checker admits (C2) |
| CT-RPI-33 | windows | The on-screen half (C9) | - |
| CT-RPI-34 | windows | The on-screen half (C9) | - |
| CT-RPI-36 | source | Now (C2) | - |
| CT-RPI-37 | synthetic | Now (C2) | - |
| CT-BND-15 | source | The review record naming each entry digest: BM5's review, when the tools ship | The 600-cell-Full, representations and jumbling tools (C5, C6, C8) |
| CT-BND-17 | source | A job that names its cap through the command layer (KN5, LP2); the repair-list inspector library entry, not written yet. Built: the repair list (pull request 176) | The kernel half on `observe::cuts::repair_list` (C8); the checker half, the built-in column's repair-list exception (LP2's pull request 229, `c600-check/tests/exceptions.rs`) |
| CT-BND-18 | source | The setup-search library entry, not written yet (collection types and word operations are in the IR draft, pull request 176) | The checker half: the built-in column's setup-search exception (LP2's pull request 229, `c600-check/tests/exceptions.rs`) |
| CT-BND-19 | source | Now (C5: the tool's views, and the live twist through the input router on the tool's view model) | - |
| CT-BND-20 | windows | The on-screen half (C9) | A held chord commits once; the runtime gets no token (C1) |
| CT-RTE-04 | source | A child's bytes without a row of their own in the IR draft (`child-decode-error-releases-reservation`, LP2); `generator-nested-same-name`, `text-comparison` and `abstract-trace-charges`, amendment candidates for C10 (reasons in LP2's pull request 234); form 2's five traces, which need `lp1-exec-region-draft-1`'s executor that LP2 does not build (LP2 plan, section 3, selection 1; stated to the owner on 18 November 2026), so they are not applicable, which is no pass (PHS.63): an item for the owner at C10 | 31 of DET.65's 40 traces (C3; three after LP2's M2 meter, six after LP2's `exact_mul` and `assert`, pull request 216, three after its generator draws, pull request 219, five after its child scopes, pull request 225, one after its loops, pull request 234, two after its byte decoding, pull request 249, and three after its streamed words, pull request 250) |
| CT-RTE-07 | source | The six jumbling keys, on M5 and menu words (C8); the six store keys, on M6's store and run record (C7, LP2's pull request 235); the keys the IR draft could not express when C4 was built, checked again against LP2's exact multiplication and loops (C4); the budget keys that need a price mapping; host events in a loop or the command layer (C5); and a Linux run | 39 of LX-3's 75 frozen keys on 600-cell-Full (C4) |
| CT-RTE-08 | source | Now: module closure (C4); persisted words (C7) | - |
| CT-RTE-10 | windows | Owner runs: LX-2 (C9) | The headless fault half (C3) |
| CT-RTE-11 | performance | Owner runs: LX-2 (C9) | - |
| CT-RTE-12 | performance | Owner runs: LX-4 (C9) | - |
| CT-RM-02 | source | Now (C2) | - |
| CT-RM-03 | source | Now (C2) | - |
| CT-RM-04 | source | Now (C2) | - |
| CT-RM-05 | windows | The on-screen half (C9) | - |
| CT-RM-06 | windows | The on-screen half (C9) | - |
| CT-RM-07 | source | Rows 9 to 14 of RM.37's jumbling vectors on M5's model: J10's exposed-face geometry (LP2); ν and the correction catalogue (pull requests 201 and 206) and covers, repair and release order (pull request 176) are built, and C8 does not use them for these rows yet | The 600-cell-Full half (C2); rows 1 to 8 and 15 of the jumbling half on M5's model (C8) |
| CT-RM-09 | source | A device for the device parts of rows 3 to 6, 8 and 10 (C9) | Rows 1, 2, 7 and 9, and the headless parts of rows 3 to 6, 8 and 10 (C2) |
| CT-RM-10 | windows | The on-screen half (C9) | - |

## 10. Stop condition for LP3, and questions to the owner

LP3's offline work stops and reports when every remaining milestone waits for a command-layer row (KN5), an LP2 item not
built yet (such as exposed-face geometry or a jumbling word key), an IR feature (the features CT-RTE-04's traces need),
the on-screen half (C9), an owner run or an owner answer, or when an implementation would change a frozen file (PHS.52).
A failing acceptance item blocks LP3's exit (PHS.63); nothing is cut, and no bar moves.

Questions are batched (owner, 16 November 2026): at most one card per milestone, each with a recommended option.

- C5: answered by the owner on 17 November 2026
  ([decision](../../wiki/decisions/owner-decisions-2026-11-17-c5-evidence.md)): LB1.10's `trace-key/1` is the draft key
  of 600-cell-Full words and freezes with the IR, and the jumbling letters come later; a receipt is not evidence (same
  page).
- C1 to C4: none. The command table's choices are drafts that freeze at LP3.
- C9: none; the owner-machine runs follow PHS.64 and are done together.
- C10: at the exit, H-08's freeze and the registration of the draft codes in chapter 13 (PHS.52, PHS.53), and
  CT-RTE-04's form-2 traces and amendment candidates (section 9).
