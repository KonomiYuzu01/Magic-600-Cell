# LP3 conformance index: notes

`tools/language/ct_index.py --phase LP3` computes the index of chapter 12 section 15.4 (each LP3 conformance test with
its state) from the table, the PHS.62 markers in the test sources and this file. This file holds what a marker cannot
say. LP3's integrator is its only editor; each note cites the record it comes from, and a state with no record has no
note.

The tool refuses (exit 2) a note for an ID outside the phase's table, an unknown note kind and a second note for one
ID. This file uses three kinds:

- `whole by union`: the parts of the ID's tests together run the whole definition, so it counts as whole.
- `indexed elsewhere`: an owner-approved index change (PHS.52) places what LP3's products cannot run with the work that
  builds it; the text says where. It applies to an ID that does not run whole, and it is no pass (PHS.63).
- `not applicable`: the half that the text names does not apply to the build; it is no pass (PHS.63).

For an ID with part markers, the index keeps the markers' not-run text beside the note. The tool's other kinds,
`waiting` and `missing despite name`, are not used here.

The rows rest on the LP3 plan's section 9 on 013900ff (`docs/progress/1.0/language-lp3-plan.md`), which reads each
test by hand. Where a test needs the owner's machine, they rest on the C9 sitting's records in
`work/experiments/c9-sitting/results/`, made with the product build of 013900ff and analysed on 22 November 2026 (pull
request 411) under the [owner's decision of that day](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md)
(items 1 to 5 and 10). The tests that these records make whole cite them in their markers. The Linux run that section 9
asks for CT-RPI-19 and CT-RTE-07 is the "Linux Rust tests" push run 44850916378 on 013900ff (CT-RPI-19: lines 1282 to
1286 of its log).

The rows below the union rows record the owner's readings (a) to (c) of the 21 tests that do not run whole, from the
owner's sign-off of C10 and LP2's exit on 22 November 2026
([decision](../../wiki/decisions/owner-decisions-2026-11-22-c10-lp2-exit.md)); every reading takes its recommended
option, and the last section has reading (d). Before them, at `798dc877`, the tool gave 61 whole (19 by union), 11 part
and 10 missing. With them it gives 61 whole (19 by union), 1 not applicable and 20 indexed elsewhere; none of the 21 is
a pass (PHS.63).

Reading (a) is an index change under PHS.52. "The post-freeze on-screen line" below is the line that builds the
on-screen contract after the freeze: it builds the device harnesses and RPI.77's quarantine, then runs reading (a)'s
halves in one owner-machine sitting. C9's parts stay on record, and nothing in reading (a)'s halves counts as passed in
LP3. Four of these IDs are named only in documentation, and their rows say so; they counted as missing before.

| ID | note kind | text |
| --- | --- | --- |
| CT-RPI-04 | whole by union | the missing source map in `c600-display/tests/plan.rs` and the older completion that cannot replace a newer one in `publication.rs` together run the whole definition (LP3 plan section 9) |
| CT-RPI-22 | whole by union | the limit half in `c600-view/tests/view_rows.rs` and the chart half in `c600-display/tests/ct_rpi_22.rs` together run the whole definition (LP3 plan section 9) |
| CT-RPI-36 | whole by union | the layer half in `c600-command/tests/view_rows.rs`, the view model's half in `c600-view/tests/view_rows.rs` and both rows through the command layer onto the view model in `c600-tools/tests/view_host.rs` together run the whole definition (LP3 plan section 9) |
| CT-EV3-06 | whole by union | the jumbling half in `c600-jumble-tool/tests/ct_ev3_06.rs` (off the lattice distinct from unknown, the occupant at lattice poses, never an old occupant, a partial relation's coverage) and the counter half in `c600-repr/tests/ct_ev3.rs` together run the whole definition (LP3 plan section 9) |
| CT-EV3-07 | whole by union | a view's open and close writing nothing on the layer in `c600-command/tests/view_rows.rs`, the checker's view-cannot-write half in `c600-repr/tests/ct_ev3.rs`, and in `c600-durable/tests/kn5_on_layer.rs` the workspace command in the session's one durable order and a view that cannot write or make undo stale, on the layer over M6's journal, together run the whole definition (LP3 plan section 9); as the durable markers say, a finished job stands in there for a view's evaluation, and H-08's own workspace rows, which the layer does not build, are not run |
| CT-BND-19 | whole by union | the layer that holds nothing for a twist before its press commits in `c600-command/tests/twist_apply.rs`, the live twist through the input router in `c600-input/tests/twist.rs` and the tool's views that hold a word's values only for the named word in `c600-tools/tests/views.rs` together run the whole definition (LP3 plan section 9) |
| CT-LB1-01 | whole by union | F2 and F5 ranks and solutions and exact Q(√5) signs in `c600-library/tests/ct_lb1_01.rs` and quaternion factor reconstruction and tree outputs against trees.py's pinned answers in `ct_lb1_01_trees.rs` together run the whole definition (LP3 plan section 9) |
| CT-LB1-02 | whole by union | the 600-cell-Full half, the library's entry "Word equality keys", in `c600-tools/tests/ct_lb1_02.rs`, LG2.18's jumbling half on a synthetic menu in `c600-run/tests/menu_word_keys.rs`, and the entry `menu_word_keys` with its key and kernel halves on M5's states in `c600-jumble-tool/tests/lb3.rs` together run the whole definition (LP3 plan section 9) |
| CT-LB1-03 | whole by union | the 600-cell-Full half in `c600-library/tests/ct_lb1_03.rs` and the jumbling half in `c600-jumble-tool`'s `tests/ct_lb1_03.rs` (the kernel), `tests/library.rs` (the language) and `tests/rows.rs` (through the host bridge) together run the whole definition (LP3 plan section 9) |
| CT-LB1-06 | whole by union | LB1.10's Full half in `c600-run/tests/trace_keys.rs` and `c600-tools/tests/supports.rs`, with its source half in `c600-syntax/tests/conformance_lg2.rs`, and the jumbling half in `c600-jumble/tests/locality.rs`, `c600-run/tests/menu_trace_keys.rs` and `c600-jumble-tool/tests/ct_lb1_06.rs` (the keys on M5, the entry `menu_traces`, changed versions and "Word keys remain separate") together run the whole definition (LP3 plan section 9) |
| CT-LB2-03 | whole by union | the kernel half in `c600-kernel/tests/kernel.rs`, the oversize refusal and chunked transport on both routes in `c600-tools/tests/net_oversize.rs` and `process.rs`, and named work-sheet reuse in `c600-command/tests/worksheet.rs` and on M6's session store in `c600-durable/tests/ct_lb2_03_work_sheet_reuse.rs` together run the whole definition (LP3 plan section 9) |
| CT-LB3-01 | whole by union | the kernel's facts in `c600-jumble-tool/tests/lb3.rs`, the library entries `jumble_end_tests` over the three end-test rows in `tests/rows.rs`, and the checker half, LP2's, in `c600-syntax` and `c600-run`'s `tests/end_tests.rs` together run the whole definition (LP3 plan section 9) |
| CT-LB3-03 | whole by union | the kernel half in `c600-jumble-tool/tests/lb3.rs` and the library entry `jumble_heights` in `tests/library.rs` and through the host bridge in `tests/rows.rs`, with the agreement with LB1 and `trees.py` on ENG-0J item 8's states and H6030 that M5's `tests/test_jumbling_rust_candidate.py` runs on the same kernel API (CT-KN2-02's `assert_heights` in `FixtureComparisonTests` and `H6030Tests`), together run the whole definition (LP3 plan section 9) |
| CT-LB3-06 | whole by union | the kernel half on exact word fixtures in `c600-jumble-tool/tests/lb3.rs` and the library entries `jumble_word_checks` and `jumble_protection` in `tests/library.rs` and through the host bridge in `tests/rows.rs` together run the whole definition (LP3 plan section 9) |
| CT-LB3-07 | whole by union | the checker half in `c600-check/tests/built_in.rs` and `c600-tools/tests/lb3_checker.rs` and the jumbling half in `c600-jumble-tool`'s `tests/lb3.rs` (the kernel and its correction part), `tests/ct_lb3_07.rs` (the inspector entries), `tests/rows.rs` (the entries at the layer's head), `tests/jumble_layer_verdicts.rs` (Jumble off) and `tests/jumble_on_derivation.rs` (Jumble on) together run the whole definition (LP3 plan section 9) |
| CT-LB4-03 | whole by union | the 600-cell-Full half in `c600-repr/tests/ct_lb4_03.rs` and the jumbling half in `c600-jumble-tool/tests/ct_lb4_03.rs`, at lattice poses other than the identity and at a later revision of a jumbled session, together run the whole definition (LP3 plan section 9) |
| CT-RTE-08 | whole by union | module closure and the module, branch, preview-outcome and in-memory persisted-word cases in `c600-module/tests/ct_rte_08.rs` and the persisted word read back from M6's session store in `c600-durable/tests/ct_rte_08_persisted_word.rs` together run the whole definition (LP3 plan section 9) |
| CT-RTE-11 | whole by union | the LX-2 fault windows in `c600-lx2` (`u7-final-lx2-20261122.json`, `verdict` met: at least 110 E2 commits in every fault; the echo bar met, with alloc_ceiling's 84 samples accepted, item 5, and samples without a confirmed frame counted apart, item 10; the renderer gate met in the three metered faults and not required in the four that lose the runtime, item 4) and the three cold startups in `tools/perf/c9/cold_start.py` (`e3-final.json`, `verdict` met, 1.890 to 1.905 s against 3.0 s) together run the whole definition (pull request 411; owner, 22 November 2026) |
| CT-LG4-09 | whole by union | LX-2's half in `c600-lx2`, the fault programs run as the shell's view jobs while it took commits and echo samples (`u7-final-lx2-20261122.json`), and LP3's half in `c600-lx4`, the editor's views open and re-run with 600 E2 turns in the same runs and charter E2 met in them (`u8-final-lx4-20261122.json`, `u8-final-e2.json`), each `verdict` met, together run the whole definition (pull request 411; owner, 22 November 2026, items 1, 2 and 10) |
| CT-ONE-26 | indexed elsewhere | its on-screen half, the presented-preview binding on the selected renderer, which needs a device harness that `013900ff` lacks: indexed in the owner-machine sitting of the post-freeze on-screen line; the headless half in `c600-command/tests/kn5.rs` and `kn5_stages.rs` runs (reading (a)) (owner, 22 November 2026) |
| CT-RPI-13 | indexed elsewhere | the whole test, picks against the drawing pass on a device, which needs a device harness that `013900ff` lacks, and no test runs a part of it: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RPI-21 | indexed elsewhere | the whole test, which needs a device harness that `013900ff` lacks, and no test runs a part of it: indexed in the owner-machine sitting of the post-freeze on-screen line; C9's U1 ran the core's removal child (`child device-loss`), a core part, which stays on record (reading (a)) (owner, 22 November 2026) |
| CT-RPI-24 | indexed elsewhere | the whole test, which needs a device harness that `013900ff` lacks, and no test runs a part of it: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RPI-31 | indexed elsewhere | `c600-display`'s `src/lib.rs` and `tests/ct_rm_09.rs` name it only for work they leave to a device, skipping a stopped frame's work, and no test runs a part of it (LP3 plan section 9); the test needs a device harness that `013900ff` lacks: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RPI-33 | indexed elsewhere | `c600-display/tests/ct_rpi_18.rs` names it only as the vertex stage it does not run, and no test runs a part of it (LP3 plan section 9); the test needs a device harness that `013900ff` lacks: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RPI-34 | indexed elsewhere | `c600-display`'s `src/lib.rs` and `tests/ct_rm_09.rs` name it only for injecting and detecting a device fault, which they leave to a device, and no test runs a part of it (LP3 plan section 9); the test needs a device harness and RPI.77's quarantine, which `013900ff` lacks: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RM-05 | indexed elsewhere | the whole test, which needs a device harness that `013900ff` lacks, and no test runs a part of it: indexed in the owner-machine sitting of the post-freeze on-screen line; C9's U1 ran the core's removal child (`child device-loss`), a core part, which stays on record (reading (a)) (owner, 22 November 2026) |
| CT-RM-06 | indexed elsewhere | the whole test, the presented-preview binding on screen, which needs a device harness that `013900ff` lacks, and no test runs a part of it: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RM-10 | indexed elsewhere | `c600-display`'s `src/lib.rs`, `src/raster.rs` and `tests/ct_rm_09.rs` name it only for a backend's work on a device, U on the device and a stopped frame's skipped work, and no test runs a part of it (LP3 plan section 9); the test needs a device harness that `013900ff` lacks: indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-BND-20 | indexed elsewhere | its actual Windows half, which needs a device harness that `013900ff` lacks: indexed in the owner-machine sitting of the post-freeze on-screen line; the headless halves in `c600-command/tests/kn5.rs`, `c600-input/tests/repeat.rs` and `c600-tools/tests/bridge.rs` run (reading (a)) (owner, 22 November 2026) |
| CT-EV3-08 | indexed elsewhere | its device half, RM.39's measured ceilings on the selected renderer: indexed in the owner-machine sitting of the post-freeze on-screen line; the admission semantics run, also through LP2's view rows on the view model (LP3 plan section 9) (reading (a)) (owner, 22 November 2026) |
| CT-RPI-08 | indexed elsewhere | its device half, J01 and J06 to J12 as a backend's own supported rows and refusals: indexed in the owner-machine sitting of the post-freeze on-screen line; the 600-cell-Full rows, J02 to J05 on M5's states, the producers of J01, J06, J07, J09, J11 and J12 and J10's payload run (LP3 plan section 9) (reading (a)) (owner, 22 November 2026) |
| CT-RM-09 | indexed elsewhere | its device half, the device parts of rows 3 to 6, 8 and 10: indexed in the owner-machine sitting of the post-freeze on-screen line; rows 1, 2, 7 and 9 and the headless parts of the others run (LP3 plan section 9) (reading (a)) (owner, 22 November 2026) |
| CT-LB4-04 | indexed elsewhere | its on-screen half: indexed in the owner-machine sitting of the post-freeze on-screen line; admission and the CPU half run (LP3 plan section 9) (reading (a)) (owner, 22 November 2026) |
| CT-RPI-32 | indexed elsewhere | its device half, the host's attachment of user stages: indexed in the owner-machine sitting of the post-freeze on-screen line; the headless part runs on stages LP2's checker admits (LP3 plan section 9), and C9's U5 ran user stages on the hardware for the renderer gate only, which stays on record (reading (a)) (owner, 22 November 2026) |
| CT-ONE-25 | indexed elsewhere | charter E2 with the tools' views open, in the owner's runs: C9's E2 runs met E2 with no tool view open, so they did not show the condition the test names; they stay on record, and the test is indexed in the owner-machine sitting of the post-freeze on-screen line (reading (a)) (owner, 22 November 2026) |
| CT-RPI-26 | indexed elsewhere | its owner-run half, charter E2 with program views active and the renderer gate: C9's E2 runs met E2 with no tool view open, so they did not show the condition the test names; they stay on record, the half is indexed in the owner-machine sitting of the post-freeze on-screen line, and the counter half runs (LP3 plan section 9) (reading (a)) (owner, 22 November 2026) |
| CT-BND-15 | indexed elsewhere | BM5's review and the review record naming each entry digest: indexed where the tools ship, as LP2's readings 9 to 16 do; the 600-cell-Full, representations and jumbling tools' halves run (LP3 plan section 9) (reading (b)) (owner, 22 November 2026) |
| CT-LB6-03 | indexed elsewhere | BM5's review and its record: indexed where the tools ship, as LP2's readings 9 to 16 do; the 600-cell-Full, representations and jumbling tools' halves run, and LX-3's cases that need word operations, which the jumbling tool's marker leaves to C4 (A08, A17 and A18: a word's length, inverse and concatenation), run in C4's file for CT-RTE-07 (LP3 plan sections 6 and 9) (reading (b)) (owner, 22 November 2026) |
| CT-RTE-07 | not applicable | B12's five cache states other than cold (warm, evicted, poisoned, shared, relocated): not applicable while the owner's decision of 20 November 2026 stands (item 1: the host keeps no row-answer cache), and they reopen if a cache is built; that decision records B12 as run cold, and the other 74 of LX-3's 75 frozen keys run (LP3 plan section 9) (reading (c)) (owner, 22 November 2026) |

## Reading (d) and LP2's IDs

PHS.23 item 1's open halves are the three tools' entries through the actual key route on Windows and the jumbling
tool's entries through the command and source routes, with their program-job route (LP3 plan section 1). They belong to
an acceptance item, not to a test of section 15.4, so no row above rests on them. The owner's reading (d): each half is
indexed with the work that builds it.

Reading (a) also moves LP2's owner-machine halves of reading 17 (CT-KN4-01, CT-KN5-05, CT-LG4-08, CT-PS1-05,
CT-ONE-20, CT-DET-05, CT-DET-07 and CT-DET-16) to the post-freeze on-screen line. They are LP2's IDs, so
`lp2-ct-index-notes.md` carries them.
