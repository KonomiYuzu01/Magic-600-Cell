# LP3 conformance index: notes

`tools/language/ct_index.py --phase LP3` computes the index of chapter 12 section 15.4 (each LP3 conformance test with
its state) from the table, the PHS.62 markers in the test sources and this file. This file holds what a marker cannot
say. LP3's integrator is its only editor; each note cites the record it comes from, and a state with no record has no
note.

The tool refuses (exit 2) a note for an ID outside the phase's table and an unknown note kind. This file uses two kinds:

- `whole by union`: the parts of the ID's tests together run the whole definition, so it counts as whole.
- `missing despite name`: only documentation names the ID, and no test runs a part of it, so it counts as missing.

The tool's other kinds, `not applicable`, `waiting` and `indexed elsewhere`, are no pass (PHS.63). This file uses none
of them while the readings in the last section wait for the owner.

The rows rest on the LP3 plan's section 9 on 013900ff (`docs/progress/1.0/language-lp3-plan.md`), which reads each
test by hand. Where a test needs the owner's machine, they rest on the C9 sitting's records in
`work/experiments/c9-sitting/results/`, made with the product build of 013900ff and analysed on 22 November 2026 (pull
request 411) under the [owner's decision of that day](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md)
(items 1 to 5 and 10). The tests that these records make whole cite them in their markers. The Linux run that section 9
asks for CT-RPI-19 and CT-RTE-07 is the "Linux Rust tests" push run 44850916378 on 013900ff (CT-RPI-19: lines 1282 to
1286 of its log).

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
| CT-RPI-31 | missing despite name | `c600-display`'s `src/lib.rs` and `tests/ct_rm_09.rs` name it only for work they leave to a device, skipping a stopped frame's work; section 9 lists no part that runs now (LP3 plan section 9: the on-screen half, C9) |
| CT-RPI-33 | missing despite name | `c600-display/tests/ct_rpi_18.rs` names it only as the vertex stage it does not run; section 9 lists no part that runs now (LP3 plan section 9: the on-screen half, C9) |
| CT-RPI-34 | missing despite name | `c600-display`'s `src/lib.rs` and `tests/ct_rm_09.rs` name it only for injecting and detecting a device fault, which they leave to a device; section 9 lists no part that runs now (LP3 plan section 9: the on-screen half, C9) |
| CT-RM-10 | missing despite name | `c600-display`'s `src/lib.rs`, `src/raster.rs` and `tests/ct_rm_09.rs` name it only for a backend's work on a device, U on the device and a stopped frame's skipped work; section 9 lists no part that runs now (LP3 plan section 9: the on-screen half, C9) |

## Proposed, owner card pending

The integrator's combined card for C10 and LP2's exit proposes the readings below. None is signed, so no row above
records one, and the tool counts each of these tests by its markers: part or missing. When the owner signs, a reading
becomes one row per ID, with the kind that the tool knows for it.

- (a) Groups c and d of the C10 sign-off package: an index change under PHS.52 moves the on-device halves to the line
  that builds the on-screen contract after the freeze, beside the stutter fix and the confirmation-gap fix. C9's parts
  stay on record, and nothing in these tests counts as passed in LP3 (`indexed elsewhere`).
  - Group c, E2 met but the condition the test names not shown: CT-ONE-25 (missing) and CT-RPI-26 (part).
  - Group d, the windows tests: CT-ONE-26 and CT-BND-20 (part); CT-RPI-13, CT-RPI-21, CT-RPI-24, CT-RPI-31,
    CT-RPI-33, CT-RPI-34, CT-RM-05, CT-RM-06 and CT-RM-10 (missing).
  - Group d, the device halves: CT-EV3-08, CT-RPI-08, CT-RM-09, CT-LB4-04 and CT-RPI-32 (part).
  - LP2's owner-machine halves, which LP2's reading 17 indexed in LP3's owner-machine batch and the C9 records do not
    name either: CT-KN4-01, CT-KN5-05, CT-LG4-08, CT-PS1-05, CT-ONE-20, CT-DET-05, CT-DET-07 and CT-DET-16. They are
    LP2's IDs, so LP2's notes carry any change to them.
- (b) Group e, BM5's review record "when the tools ship": CT-BND-15 and CT-LB6-03 (part), indexed where the tools ship,
  as LP2's readings 9 to 16 do (`indexed elsewhere`), or left `waiting` in LP3.
- (c) Group f: B12's five cache states in CT-RTE-07 are not applicable while the owner's decision of 20 November 2026
  (item 1) stands, and reopen if a cache is built. That is no pass, so CT-RTE-07 stays part either way; its marker in
  `c600-module/tests/ct_rte_07_full.rs` names B12 as not run.
- (d) PHS.23 item 1's open halves, the three tools' entries through the actual key route on Windows and the jumbling
  tool's entries through the command and source routes with their program-job route: each indexed with the work that
  builds it, or headless tests now for the jumbling routes. This is an exit item, and no row above rests on it.
