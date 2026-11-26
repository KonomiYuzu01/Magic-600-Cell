# ADR-D02 shell: build-direction framework (revision 2)

Status: revision 2, written by Fable as a subscription subagent on 26 November 2026 after the owner's three answers of
the same day ([decision](../../wiki/decisions/owner-decisions-2026-11-26-adr-d02-build.md)). The build, the owner's
view and every acceptance check are in the product shell, with Look Lab dropped and Jumble drawing built in this build.
J0 and J1 gain one lean researcher subagent. On a second card the owner chose two drawn views, Global and Local at
once, and let the drawing be built and measured before R5. Fable's addition for the second view is spliced in: steps
B18a to B18c, gate rows G9 to G11, and findings F15 to F18. B1 waits for the owner's go. Revision 2 replaces revision 1
(commit `8f39b4d5`: 23 steps in Look Lab's chrome library with a live Look Lab window).

Owner decision behind the framework: [25 November 2026](../../wiki/decisions/owner-decisions-2026-11-25.md), item 5.
Fable frames, Opus builds one step per commit, and Fable reviews at checkpoints, on the subscription with no API.
The record it builds is [ADR-D02](../../architecture/1.0/design/adr/ADR-D02-jumble-research.md), as corrected on this
branch. The integrator's check and dispositions are in section 10. Apart from section 10, the text is Fable's, with one
source filled in section 6 (marked) and a local path removed.

This is the framework only: nothing has started. Fable changed no file and ran no build, test, shell or cargo command.
Everything below comes from reading tracked source, so each finding is a lead until a test confirms it. Every
relative path is from the repository root.

## 1. Overview

**Shape (36 steps, 6 checkpoints, 2 gate sittings).**
1. B1 to B6, foundations: adapter samples, design data, host primitives, the shell's D02 switch and scripted runner, session fixtures at the sample states, the Jumble host.
2. B7 to B13, the Solve side: totals, cap map, threshold and naming, twist checks, covers, repair list, staging and the boundary.
3. B14 to B19, drawing part 1 in the core: the style path, tints on pieces, line overlays, picks, the sweep, two drawn views (B18a to B18c), gate sitting 1.
4. B20 to B24, drawing part 2: the cut slice, the raw view, the two chart presets, gate sitting 2.
5. B25 to B31: words, the ν band, keys and terms, the research pane, the tuning panel.
6. B32 and B33: the reduced Track J run, with one lean researcher subagent.

**Where each surface lives.**
- **Chrome library:** it stays at `tools/looklab/app/Chrome/` (namespace `Magic600.Chrome`). The shell already stages it, so this is the least moving that keeps one copy.
  - `Model/` is pure C#. Its console tests stay in `tools/looklab/tests/` and run with `python tools/looklab/check.py`: no Godot, no Look Lab window.
  - `Ui/` holds the Godot controls. Look Lab still compiles them, but it attaches no D02 surface and is no part of acceptance.
  - `Ui/JumbleSurface.cs` is the one assembly point the shell calls.
- **Product shell** (`renderer/shell/`): the build surface, the owner's view and every acceptance check. D02 wiring goes in a new folder `renderer/shell/project/D02/`.
- **Host library** (`renderer/bridge/host`): the state digest, session fixtures at arbitrary words, and the style, overlay, pick and stage calls.
- **Core** (`renderer/core`): the drawing itself, on the product pose path that already draws posed pieces (row J02). From B18a, one composite frame draws up to four panes on one surface with one present, so Global and Local are drawn at once.
- **Facts:** the chrome reads sample files through `IJumbleSource`. K2 later binds the same interface to `m6h_invoke`. The session is real: after a twist the state has no sample, so slots go stale until an undo returns to it.

**How the owner watches.**
- Opus ends every step with a product build: `python renderer/shell/build_product.py`.
- The owner runs one command from a PowerShell on the desktop: `python renderer/shell/watch_d02.py [--state <id>]`. It picks the newest build and a fresh run folder and calls `launch.py --frame --puzzle jumble-on` with the D02 options.
- There is no live reload. The owner closes and reruns the command after a step lands.
- Each step's "Owner sees" line says what to look at. Each checkpoint has a short sitting.

**Machine marks.**
- **H:** headless; no window, no GPU.
- **W:** a shell window on the owner's machine, with the GPU, for seconds to minutes. Scripted runs may start from Opus's process, because they inject their keys inside the shell.
- **G:** a gate measurement: the GPU, a quiet machine, an attended start by the owner.

**Every step.**
- Run `python tools/looklab/check.py` (console tests) and `python tests/test_renderer_shell.py`.
- Run `python tools/wiki/lint.py` when design data changed.
- New console tests are `tools/looklab/tests/D02*Tests.cs`, registered in `tools/looklab/tests/Program.cs`.
- Round 3 values are labelled placeholders; icons are tier 1 glyphs.
- Everything D02 in the shell sits behind `M600_D02=1`, so measured C9 runs are unchanged.

## 2. Steps

### B1. Sample data from the Rust adapter
- **Goal:** tracked sample files for every state, rebuilt only through the adapter, each with its digests.
- **Items:** none directly.
- **Files (new):** `tools/looklab/fixtures/make_d02_samples.py`; `tools/looklab/data/samples/jumble/`; `tests/test_d02_samples.py`.
- **Add:**
  - `build_state(spec)` over `RustJumbleEngine`: `start`, `identity`, `word` per segment (asserting `status == 'admissible'` and `applied == len(word)`), `record`.
  - Per state: `totals.json`, `rim.json` (600 rows), `contact.json` (domains, contact edges with off-grip contacts), and `style.json` (per-pose domain, height and lattice flag, in the canonical pose table's order).
  - Per named cap, `cap-<id>.json`: covers, the repair list in three variants (completed, cut off, cancelled), twelve release checks of one seat, straddlers, the certificate-point count.
  - Per named cap, piece masks of 177,120 bits: inside set, straddlers, hidden pieces, and the moving sets of repair rows 1 to 9.
  - Three digests in every state's header: the adapter's `record()['digest']`, the SHA-256 of the `pose_index` array, and that of `pose_lattice`. The last two bind a sample to a shell session (B5).
  - Modes `--write` and `--verify` (byte equality). Format `magic600-d02-sample/1`.
- **Decide first:** assert that `start(I_a)` then `record()['digest']` equals the W-J fixture's `stages.start.digest`. If it does not, the seed 7001 chain compares through `j1_from_record`. Record the case in `states.json`.
- **Check:** `python tests/test_d02_samples.py` (no adapter) checks the format, section 6's digests and counts, and that the rim classes sum to 600. `make_d02_samples.py --verify` prints `D02_SAMPLES_PASS states=<n>`.
- **Owner sees:** nothing yet.
- **Machine:** H. About two minutes per deep state.
- **Checkpoint:** no.
- **Depends:** the adapter build named in the packet.

### B2. Design data: slots, patterns, motion version 2, rows, words
- **Goal:** every slot, pattern, motion row, draft row and sentence exists as data first.
- **Items:** .1, .27, .37, .41.
- **Files:** in `tools/looklab/data/design/`: `system.json`, both themes, `motion.json`, new `command-rows-d02.json` and `jumble-words.json`, `command-labels.json`, `command-icons.json`; new `tools/looklab/tests/D02DataTests.cs`.
- **Add:**
  - Roles mapped in both themes: `numeral.undefined`, `commit.bar.whole_only`, `mark.practice`, `mark.jumble`, `slot.tool.state.*` (seven), `pattern.cap.*`, `pattern.cut.*`, `pattern.preview`, `tint.jumble.contact_class.{1..6, overflow}`, `tint.jumble.height.{2, 4, 6, above}`, `material.exposed_face`, `material.exposed_face.tinted`, `state.exposed_face.unavailable`.
  - Motion version 2: `slice.turn`, `jumble.sweep`, `jumble.half_turn`, `jumble.enter`, `jumble.leave`, `repair.step`, `cap.name`, and `slot.fill` (off, cue `cue.check` reserved).
  - Draft rows, each with its `request` (CR-14 to CR-17, CR-19).
  - Every sentence the record quotes, and the display names.
- **Check:** `D02RolesExistInBothThemes`; `D02MotionRowsStaticUnderReducedMotion`; `D02DraftRowsAbsentFromTable` (fails the day a row lands, so the draft row is deleted); `D02WordsMatchRecord` (byte equality).
- **Owner sees:** nothing yet.
- **Machine:** H.
- **Checkpoint:** no.
- **Depends:** none.

### B3. Host primitives in the chrome model
- **Goal:** the revision 6 rules ADR-D02 rests on; they are absent today (F1).
- **Items:** .7, .40.
- **Files (new):** `Model/ToolSlots.cs`, `Ui/ToolSlotBox.cs`, `tests/D02SlotTests.cs`.
- **Add:**
  - `enum SlotState` (seven states), `enum JobEnd { Completed, Cancelled, CutOff, Error }`, `record Coverage`, `record ToolResult`.
  - `class ToolSlot` with `Request(release, digest, parameters)`, `Finish`, `Cancel`, `Invalidate(digest)`, `Word(words)`. Rules:
    - only the latest release with matching digest and parameters is taken;
    - a completed result without coverage is `Error`;
    - cancelling the latest restores the previous state;
    - an older release changes nothing.
  - `class ReleaseRecord` with `Next()`, `TryNumber(cap, revision)`, `Superseded(release)`.
  - `Numeral` with `NumeralKind { Value, Undefined, Pending, Incomplete }`.
  - `JumbleSlots`: the eight slot ids.
- **Check:** `D02SlotLateResultIgnored`; `D02SlotCancelLatestRestores`; `D02SlotCancelOlderChangesNothing`; `D02SlotCompletedWithoutCoverageIsError`; `D02SlotWordsCoverSevenStates`; `D02NumeralKindsStayApart`.
- **Owner sees:** nothing yet.
- **Machine:** H.
- **Checkpoint:** no.
- **Depends:** B2.

### B4. The shell's D02 switch, staging, scripted runner and watch command
- **Goal:** the shell stages D02's data, opens with D02 on, can be driven by a script, and has one command for the owner.
- **Items:** none new.
- **Files:**
  - `renderer/shell/build_shell.py`, `renderer/shell/launch.py`, `renderer/shell/project/ShellChrome.cs`;
  - new `renderer/shell/project/D02/ShellJumble.cs`, `renderer/shell/watch_d02.py`, `renderer/shell/d02_run.py`;
  - new `tools/looklab/app/Chrome/Ui/ScreenText.cs`;
  - `tests/test_renderer_shell.py`.
- **Add:**
  - `build_shell.py` stages `tools/looklab/data/samples` to `project/samples`.
  - `launch.py` gains `--d02` (sets `M600_D02=1`; needs `--frame`), `--d02-fixture DIR` (records only its SHA-256, as `--lx4-fixtures` does) and `--d02-script FILE`.
  - After the registry at `ShellChrome.cs:124`, when D02 is on: add the draft rows, relabel, and call `ShellJumble.Attach`. A draft row shows a "draft row" mark in the palette.
  - `ScreenText.Of(Control)`: the visible text of the frame, pane by pane.
  - A script player in `ShellJumble`. It feeds each step's chord to the frame's own key entry (`ShellChrome.Takes`, then the session router), writes `d02-screen.jsonl` after each step, and quits.
  - `d02_run.py --script <name> --out <new folder>` launches, compares each dump with the expectation Python computes from the sample files, and prints `M600_D02_RUN_PASS script=<name> steps=<n> mismatches=0`.
- **Check:** new cases in `tests/test_renderer_shell.py`: D02 staging; the three launch options over the fake build; no D02 variable without `--d02`; no private path in `launch.json`. Then `python renderer/shell/d02_run.py --script smoke`.
- **Owner sees:** the frame opens with D02 on; the palette lists the draft rows.
- **Machine:** H, then W for the smoke run.
- **Checkpoint:** no.
- **Depends:** B2.

### B5. Session fixtures at the sample states, and the state digest
- **Goal:** the shell opens a real Jumble session at a sample state, and the chrome knows which sample matches it.
- **Items:** none new.
- **Files:**
  - `renderer/bridge/host/src/fixture.rs`, `src/bin/c600-shell-session-fixture.rs`, `src/lib.rs`, `tests/turns.rs`;
  - `renderer/shell/project/HostLibrary.cs`;
  - new `renderer/shell/build_d02_fixtures.py`;
  - `tests/test_renderer_shell.py`.
- **Add:**
  - `fixture::write_word(dir, kernel, letters)`: a Jumble-on session whose letters are committed as `jumble.twist.menu` through the command layer, with the correctness control at the close. Today's writer only presses fixed router keys on one cap (F5).
  - The binary's `--word <file>` option.
  - Export `m6h_state_digest(handle, out)`, giving the head's digest and the two pose-array digests. Host ABI 8; move the pin in `HostLibrary.cs`.
  - `build_d02_fixtures.py` writes one fixture per sample state under `work/build/d02-fixtures/<state>/` and checks the pose-array digests against the sample's.
  - `ShellJumble.Bind()`: at every adopted revision, load the sample whose pose-array digests equal the session's; with none, every slot is `Stale` and the readouts are pending.
- **Check:** `python tests/test_render_ffi.py` with new host tests `a_word_fixture_reopens_at_its_digest` and `the_state_digest_follows_undo_and_redo`. Then `d02_run.py --script bind` on `wj-ia-300`: bound at open, unbound after one twist, bound again after undo.
- **Owner sees:** `watch_d02.py --state wj-ia-300` shows the jumbled pieces, posed, under the frame.
- **Machine:** H for tests; W for the run. Record each fixture's open time: a deep fixture replays about 3,300 commits (risk R2).
- **Checkpoint:** no.
- **Depends:** B1, B4.

### B6. The Jumble host in Solve
- **Goal:** Jumble's hosting in the Solve layout, with no new layout or dock.
- **Items:** .2, .6, .11, .12.
- **Files:** new `Model/Jumble.cs`, `Ui/JumbleHost.cs`, `Ui/JumbleSurface.cs`; edits to `Ui/ChromeFrame.cs`, `Ui/SolvingSurface.cs`, `Ui/OrbitRail.cs`; `renderer/shell/project/D02/ShellJumble.cs`; new `tests/D02HostTests.cs`.
- **Add:**
  - `interface IJumbleSource`, `SampleJumbleSource`, and `JumbleState` (`Mode`, exact `LatticeFlag`, `NamedCap`, `Revision`, `SessionKind`).
  - `EntrySlot` for `view.preset.jumble_entry`: `Apply()`, and `Revert()` of only what is unchanged since. It follows the adopted revision's mode.
  - `ChromeFrame.SetModeCell`, and `mark.jumble` on every view border.
  - The strip and the rail show the undefined numeral as a whole when the lattice flag is false, never on a pending N.
  - Local's empty sentence, and a four-header accordion in Local, one open at a time, each closed header with a one-line count.
- **Check:** `D02EntryRevertsOnlyUnchanged`; `D02EntryFollowsUndoRedoBranchReopen`; `D02EntryIsChromeOnly`; `D02UndefinedSwitchesOnLatticeFlagNotPendingN`; `D02MarkOnEveryViewBorder`; `D02PanelsMoveFreely`. Then `d02_run.py --script host`.
- **Owner sees:** the Jumble cell, a mark on each border, the strip and the rail undefined over the posed W-J state; true zeros on the zero-twist state.
- **Machine:** H, W.
- **Checkpoint:** yes, C1.
- **Depends:** B3, B5.

### B7. Totals and the named-cap readouts
- **Goal:** the border numerals, with pending, incomplete and undefined apart.
- **Items:** .22.
- **Files:** new `Model/JumbleReadouts.cs`, `Ui/JumbleReadouts.cs`, `tests/D02ReadoutTests.cs`.
- **Add:**
  - `GlobalTotals`: N with its two parts, domains, off-lattice pieces, n2, n4, n6, and "above 6" with its largest height.
  - `CapReadout`: In, Out, A, N_c, floor, u.
  - Fixed tabular slots, one neutral change mark, the unit word "seams", no colour by value.
- **Check:** `D02TotalNPendingUntilSeamFactsAll`; `D02TotalNIncompleteAfterCutOff`; `D02TotalNIsNotSumOfCaps`; `D02BlockedCapReadoutUndefined`; `D02Above6MaxUndefinedAtZero`; `D02TotalsMatchSample`.
- **Owner sees:** the totals on Global's border at a deep export.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B6.

### B8. The cap map
- **Goal:** one canvas control of 600 caps in the ring-sheet arrangement, in Solve's bottom dock.
- **Items:** .13, .14.
- **Files:** new `Model/CapMap.cs`, `Ui/CapMapControl.cs`, `tests/D02CapMapTests.cs`; `make_d02_samples.py` (`layout()`, writing `cap-layout.json`).
- **Precondition:** M-RING redone. The builder maps each cap id to a row and column by matching the adapter's `model()` poles to the cells of `tools/looklab/fixtures/hopf-rings.json`. `test_d02_samples.py` checks that the map is a bijection onto 20 rows of 30, and that row neighbours are face neighbours.
- **Add:**
  - `enum CapClass { Pending, Blocked, Closed, AtFloor, AboveFloor }`, from certified and completed rim rows only.
  - One `_Draw` with the `pattern.cap.*` patterns, a wrap mark on the cyclic columns, and the legend "at floor (N_c = F_c > 0)".
  - The dock panel `capmap`, collapsed by default and droppable as a free pane.
- **Check:** `D02CapClassesExclusiveAndExhaustive`; `D02NoNumeralAtBlockedCap`; `D02UncertainRimRowIsPending`; `D02CapMapMatchesRim` (27 admissible, 573 blocked at the journal's end); `D02CapLayoutIsRingSheet`.
- **Owner sees:** 20 rows of 30 cells in the bottom dock; mostly blocked ground at the W-J end state.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B6.

### B9. Threshold, naming a cap, targets
- **Goal:** the predicates, naming, the 80 meeting caps and the inspector tiers.
- **Items:** .15, .16, .19.
- **Files:** `Model/CapMap.cs`, `Ui/CapMapControl.cs`, `Ui/JumbleHost.cs`, `Ui/InspectorPanels.cs`, new `tests/D02ThresholdTests.cs`.
- **Add:**
  - `record CapPredicate(Kind { AtLeast, AtMost, AtFloor }, int K)`.
  - A threshold set with no default. `Counts()` gives members, non-members, blocked and pending, each apart. The Query set outline. No sort member.
  - The threshold field and its count on Global's border while the dock is closed.
  - Naming by cell or by typed number; the 80 meeting caps as a static mark.
  - No slot leaves `Empty` before a cap is named; hovering only inspects.
  - Tier 1 and Tier 2 facts in a fixed order.
- **Check:** `D02NoDefaultThreshold`; `D02ThresholdCountsBlockedApart`; `D02ThresholdNeverReorders`; `D02NoJobBeforeNaming`; `D02HoverNamesNothing`; `D02MeetingCapsAre80`. Then `d02_run.py --script name-cap`, by keys only.
- **Owner sees:** type a cap number and read the three threshold counts.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B8. CR-14 for the threshold and naming rows (draft rows meanwhile). Picking a sticker comes in B17.

### B10. Kept twist checks and the Q3.b test
- **Goal:** the release-ordered list with folded rows, under J-7 and N-1 as the owner reads them.
- **Items:** .35, .36.
- **Files:** new `Model/TwistChecks.cs`, `Ui/JumbleLists.cs`, `tests/D02TwistCheckTests.cs`.
- **Add:**
  - `KeptChecks` with `Release`, `Complete`, `Cancel`; `CheckRow` and `FoldedRow`.
  - A folded row is written inside `Release` from the release record alone. A cancel of a non-latest release is a no-op.
  - `SampleJobs`: a tick clock that completes sample results.
- **Check:** `D02Q3bTwelveTwists` asserts the four points of ADR-D02.36. `d02_run.py --script q3b` repeats it in the shell and prints `screen_changes_on_cancel=0`.
- **Owner sees:** hold a seat key; rows and folded rows build with no flicker.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B3, B9.

### B11. Cap covers and the predicate focus layers
- **Goal:** the Cap covers slot, and focus layers with counts.
- **Items:** .38.
- **Files:** new `Model/CapCovers.cs`, `Model/Gallery.cs` (extend `FocusLayers`, F2), `Ui/JumbleLists.cs`, `tests/D02CoversTests.cs`.
- **Add:**
  - The four lines in cap-id order and the 80-row table, with blocked meeting caps listed as undefined.
  - `FocusLayers.AddPredicate(id, label, count)` on the existing cycle key, with three layers: straddlers, one domain, height at least k. Their drawing comes in B15.
- **Check:** `D02CoversUndefinedIsListed`; `D02CoversTableHas80RowsBothDirections`; `D02HoverInCoversMarksNoCapMapCell`; `D02FocusLayerCountsMatchSample`.
- **Owner sees:** Cap covers under the named-cap cell; the focus key shows each layer's count.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B9.

### B12. The repair list
- **Goal:** the list exactly as the 17 October exception allows.
- **Items:** .32, .34.
- **Files:** new `Model/RepairList.cs`, `Ui/JumbleLists.cs`, `tests/D02RepairTests.cs`.
- **Add:**
  - `RepairRow`, and `RepairListModel.From(sample)` in the adapter's order, with no comparer and no sort or filter member.
  - No focus on open. Digits 1 to 9 only in the context `repair.list.focused`.
  - ΔN as one tabular numeral.
  - The words "Repair k of m" (m = 2 or 3) and "Whole repair only"; the empty line with its coverage line; the cut-off line.
  - Step focus through `view.preview.scrub`, with the step's moving-set count. Its outline is drawn in B16.
- **Check:** `D02RepairOrderEqualsSample`; `D02RepairModelHasNoSortMember` (reflection); `D02RepairNoFocusOnOpen`; `D02RepairDigitsOnlyInContext`; `D02RepairEmptyNamesCapRevisionAndCoverage`; `D02RepairCutOffIsNotEmpty`; `D02RepairTwoTwistRowHasTwoSteps`.
- **Owner sees:** the repair slot; arrows focus a row; the step label follows the scrub key.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B9.

### B13. Staging a picked row, and the boundary
- **Goal:** a picked row becomes a sealed, whole-only preview; nothing is picked for the solver; research output is refused at preview.
- **Items:** .5, .33.
- **Files:** `Model/RepairList.cs`, `Model/Session.cs`, new `Model/Boundary.cs`, `Ui/DraftLine.cs`, `tests/D02BoundaryTests.cs`.
- **Add:**
  - `SealedSpan`, removed whole and never edited.
  - `Stage(row, userEvent)`, callable only from the pick handlers. It writes the repair-pair record to the sample preview and sets the whole-only bar.
  - `enum Origin { Solver, Library, Research }`. `PreviewModel.Admit` refuses `Research` in a solving session, with its sentence.
  - `BoundaryLedger`: it counts any staging, focusing, ordering or applying that carries no user event.
- **Check:** `D02StageOnlyFromPick`; `D02SealedSpanRemovedWhole`; `D02TypedTwistsAreOrdinaryWord`; `D02ResearchOriginRefusedInSolvingSession`; `D02BoundaryCountZeroOverScriptedSession`. Then `d02_run.py --script repair`.
- **Owner sees:** press 2 in the list; one sealed span appears and the bar reads "Whole repair only".
- **Machine:** H, W.
- **Checkpoint:** yes, C2.
- **Depends:** B12. CR-15: the engine's preview and record write wait for the row and K2.

### B14. The style path: core, bridge, host
- **Goal:** the core can draw per-piece style from masks. No look yet.
- **Items:** none; rows J07 and J09 of the feature matrix, as candidates (F7).
- **Files:**
  - core: `renderer/core/include/sa2_interop.h`, `src/product.cpp`, `src/product.h`, `src/scene.cpp`, `shaders/product.hlsl`, `src/selftest.cpp`, `check_core.py`, `PROVENANCE.md`;
  - bridge: `rust/c600-render` and `renderer/bridge/src/lib.rs` (the function table);
  - host: `renderer/bridge/host/src/lib.rs`;
  - shell: `renderer/shell/project/CoreLibrary.cs`, `HostLibrary.cs`;
  - tests: `tests/test_renderer_core.py`, `tests/test_renderer_shell.py`.
- **Add:**
  - Core ABI 6, additive:
    - `sa2_style_set(ctx, revision, const uint8_t* piece_style, uint32_t channel)`: one byte per piece, two channels (fill tint index, outline index), bound to a revision as the pose arrays are;
    - `sa2_view_style(ctx, view, const sa2_view_style*)`: tint mode, dim mode, a 16-entry palette, a 3×3 output colour matrix. Its `view` argument is a style slot (1 to 4) that a pane names (B18a), since the held split needs two styles on one view.
  - One precompiled product pipeline variant. The existing pipeline is unchanged when no style is set.
  - Host exports `m6h_set_piece_style` and `m6h_set_view_style` (ABI 9). The host expands per-pose sample arrays to pieces with the session's own `pose_index`.
  - Self-test mode `--style`, modelled on `--poses`, on WARP.
- **Check:** `python renderer/core/check_core.py --style`:
  - styled pieces differ from unstyled only on the masked pieces;
  - an empty mask draws byte-identically to no style;
  - a style for another revision is refused.

  Plus `python tests/test_renderer_core.py` and `python tests/test_render_ffi.py`.
- **Owner sees:** nothing new.
- **Machine:** H (WARP).
- **Checkpoint:** no.
- **Depends:** B5.

### B15. Tints on pieces: contact classes, height, off-lattice, focus dim
- **Goal:** the one tint channel, drawn on the posed pieces.
- **Items:** .20, .21, .23, .24, .30.
- **Files:** new `Model/JumbleTint.cs`, `Ui/JumbleLegend.cs`, `renderer/shell/project/D02/ShellJumble.cs`, `tests/D02TintTests.cs`.
- **Add:**
  - `enum TintMode { None, ContactClass, Height, OffLattice }`: one field, so no stacking.
  - `ContactClasses.Assign(contact, previous)`: proper, deterministic, sticky by domain id; a seventh need is the overflow state with its count; the lattice domain is neutral. Saved with the view state.
  - Drawing: contact class on the outline channel, so the fill keeps the label; the height ramp on the fill tint; off-lattice as a dark outline taken from the bundle's `pose_lattice`.
  - Focus dim through the dim mode. The straddler layer is never a default and is active only at a blocked cap.
  - Legends with counts.
- **Check:** `D02TintModesExclusive`; `D02ContactColouringIsProper`; `D02FourClassesAtWj300AndDeepExports`; `D02OneClassAtIa40`; `D02ClassStickyAcrossCommitUndoRedoBranchReopen`; `D02OverflowNeverReusesAcrossContact`; `D02OffLatticeFromFlagNotHeights`; `D02StraddlerLayerOnlyAtBlockedCap`. Then `d02_run.py --script tint`: the style bytes sent to the host equal the sample's classes, piece by piece.
- **Owner sees:** switch the tint mode from the palette on a deep export: four outline classes, the height ramp, the dim of one domain.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B11, B14. CR-14 for the mode and layer rows.

### B16. Overlays as lines
- **Goal:** the cut trace, the never-hidden set, hidden-piece outlines, repair moving sets and certificate glyphs.
- **Items:** .26.
- **Files:** core (`sa2_interop.h`, `src/product.cpp`, `src/scene.cpp`, new `shaders/lines.hlsl`, `src/selftest.cpp`); the bridge table; host `src/lib.rs`; `ShellJumble.cs`; `tests/test_renderer_core.py`.
- **Add:**
  - `sa2_lines_set(ctx, view, layer, const float* points4, count, style)`: line lists in four dimensions through the frame's chain, at most four layers. Host export `m6h_set_lines`. As in B14, `view` is a style slot that a pane names.
  - Layers:
    - the named cap's cut trace, constant per cap, from the engine's cut (J04);
    - certificate glyph pairs, off until its gate row and the owner's acceptance; the count shows in their place;
    - piece outlines from masks: hidden pieces with their exact count, and the focused repair step's moving set.
  - The never-hidden set of `jumbling-observation.md` rule 5 is forced visible in the preparation's visibility bytes.
- **Check:** `check_core.py --lines` on WARP: a line layer covers pixels only within two pixels of its projected segments; an empty layer changes nothing. Then `d02_run.py --script overlays`: hidden count equals the sample; the never-hidden set stays visible under every focus layer.
- **Owner sees:** name a cap and its cut trace appears; step a repair row and the moving set outlines.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B12, B14.

### B17. Picks in Jumble
- **Goal:** a sticker selects its piece; a cap's cell already names its cap.
- **Items:** none new (completes .19's pointer route).
- **Files:** host `src/lib.rs`, `src/turns.rs`; `renderer/shell/project/Shell.cs`; `ShellJumble.cs`; host tests.
- **Add:** with Jumble on the host marks slots pickable and resolves a hit through `c600_display::jumble::PoseProjection` to a piece id. An off-lattice piece has no slot and resolves to the piece only. A stale hit is dropped. Tier 1 shows kind, id and numerals.
- **Check:** new host tests `a_jumble_pick_resolves_to_its_piece` and `a_stale_jumble_pick_is_dropped` in `python tests/test_render_ffi.py`. Then `d02_run.py --script pick`, where an injected click at a known pixel selects the expected piece.
- **Owner sees:** click a sticker; the inspector shows its piece, domain and height.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B9, B14.

### B18. The sweep and the half-turn
- **Goal:** a committed jumble twist is swept; a half-turn cuts and outlines.
- **Items:** none new (applies .37; row J03).
- **Files:** host `src/lib.rs`, `src/turns.rs`; the core, only if product frames lack the transform; `Model/Gallery.cs` (`MotionTable`); `ShellJumble.cs`.
- **Start by reading** whether a product frame can carry `c600_display::jumble::transform`'s row. If not, add `sa2_transform_set(ctx, revision, plane_u, plane_v, angle, const uint8_t* moving_mask, phase)` as B14 added style.
- **Add:**
  - The sweep of the certified inside set, with its duration from the row `jumble.sweep`.
  - A half-turn cuts to its end pose and outlines the moved set for the row's fixed time.
  - A refused twist gets no sweep. Reduced motion makes both static.
- **Check:** host test `a_committed_jumble_twist_carries_its_certified_transform`. `check_core.py --sweep` on WARP: phase 0 equals the old pose and phase 1 the new. `d02_run.py --script sweep`: no sweep under reduced motion or on a refusal.
- **Owner sees:** twist with the keys and the inside set turns; a half-turn flashes its outline.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B14, B16.

**Two drawn views (B18a to B18c), added for the owner's answer of 26 November 2026.** Design: one surface, one present, several panes.
- The core's surface covers the bounding rectangle of the drawn openings.
- A composite frame clears once, draws each pane in its own rectangle and presents once.
- The gate is still read from one `presented` row per frame.
- Two surfaces are not used: that would mean two presents per refresh and two ledgers (F15).
- ABI numbers: core 7 and host 10, or the next free numbers if B16 to B18 moved them.

### B18a. The composite frame in the core
- **Goal:** one frame draws up to four panes at one revision; a pick resolves its pane.
- **Items:** none; feature-matrix row F27 (further views), as a candidate.
- **Files:**
  - `renderer/core/include/sa2_interop.h`, `src/product.cpp`, `src/product.h`, `src/scene.cpp`, `src/frame_target.h`, `src/selftest.cpp`, `check_core.py`, `PROVENANCE.md`;
  - `renderer/shell/project/CoreLibrary.cs` (the version pin);
  - `tests/test_renderer_core.py`.
- **Add:**
  - `SA2_PANES_MAX 4`.
  - `struct sa2_pane { uint64_t view, view_generation, resource_set; int32_t x, y; uint32_t width, height; float camera_matrix[16]; uint32_t style; uint32_t flags; }`.
  - `sa2_frame_submit_panes(ctx, const sa2_frame_input* frame, const sa2_pane* panes, uint32_t count, uint64_t* out_frame_id)`. Rules:
    - one clear of the whole target, one draw per pane with viewport and scissor at its rectangle, one present, one frame id;
    - rectangles lie inside the target and do not overlap;
    - a count of 0 or above 4 is refused;
    - every pane's resource set must belong to the frame's engine revision, else the whole frame is refused.
  - `sa2_frame_panes(ctx, frame_id, rows, capacity, out_count)`: per pane, its view, generation and vertex evaluations. The frame report's `view_vertex_evals` is the sum.
  - `sa2_pick_request` on a composite frame takes target coordinates and draws the ID pass with the camera, set and viewport of the pane that contains the point.
    - A point in no pane is refused.
    - The reply names that pane's view.
    - Only a newer generation of that view makes the reply stale.
  - A CPU self-test `pane arguments`, and a WARP mode `--panes` modelled on `--poses`.
- **Check:** `python renderer/core/check_core.py --panes`:
  - one pane covering the target is byte-identical to `sa2_frame_submit` of the same input;
  - each of two panes matches a one-view frame drawn at the pane's size (at most 1% of covered pixels differ by more than 2);
  - pixels outside every pane hold the clear colour;
  - overlap, a fifth pane and a mixed revision are refused;
  - a pick in each pane returns that pane's view and the instance a one-view pick returns;
  - the two-pane frame's vertex evaluations equal the sum of the two one-view frames'.

  Plus `python tests/test_renderer_core.py`.
- **Owner sees:** nothing new.
- **Machine:** H (WARP).
- **Checkpoint:** no.
- **Depends:** B14, B17.

### B18b. Two views in the bridge and the host
- **Goal:** the host opens Local, publishes both views from one adoption, and never frames two revisions.
- **Items:** none new.
- **Files:**
  - `rust/c600-render/src/abi.rs`, `backend.rs`, `fake.rs`, `tests/abi_layout.rs`;
  - `renderer/bridge/src/lib.rs`, `tests/ffi.rs`, `tests/abi_functions.rs`;
  - `renderer/bridge/host/src/lib.rs`, `src/turns.rs`, `tests/c_api.rs`, `tests/view.rs`;
  - `renderer/shell/project/HostLibrary.cs`;
  - `tests/test_renderer_shell.py`.
- **Start by reading** whether `c600_display`'s host already keeps a second view id with its own publication.
- **Add:**
  - `LOCAL: ViewId = ViewId(2)` beside `VIEW` (`renderer/bridge/host/src/lib.rs:161`).
  - Exports:
    - `m6h_open_local(handle)`;
    - `m6h_set_view_camera(handle, view, matrix)`;
    - `m6h_set_view_visible(handle, view, visible)`: Local can be hidden, never closed;
    - `m6h_set_pane(handle, pane, view, x, y, width, height, style)`;
    - `m6h_clear_pane(handle, pane)`.
  - One revision: at each adoption the host publishes every open view from the same bundle, with a preparation per view (visibility, interaction and instance bytes are per view already). `m6h_frame` submits a composite only when every paned view's active set is at the adopted revision; until then it re-submits the last complete pane list. `M6hFrame` gains the pane count and a `held` flag.
  - With no pane set, `m6h_frame` takes today's one-view path, so runs without `--d02` are unchanged.
  - A hidden view gets no pane and no draw.
  - B17's pick call takes surface coordinates and answers the view with the piece.
- **Check:** `python tests/test_render_ffi.py` with new host tests:
  - `two_views_publish_one_revision`;
  - `a_frame_never_mixes_revisions` (the fake backend delays one view's activation);
  - `a_hidden_local_adds_no_vertex_evaluations`;
  - `one_view_frames_are_unchanged` (the fake's call log equals today's);
  - `a_fifth_pane_is_refused`;
  - `a_pick_names_the_view_it_hit`.
- **Owner sees:** nothing new.
- **Machine:** H.
- **Checkpoint:** no.
- **Depends:** B18a.

### B18c. Two openings in the shell, and picks per view
- **Goal:** Global and Local are both drawn; a click goes to the view it lands in.
- **Items:** none new (it removes Local's "not drawn yet").
- **Files:**
  - new `tools/looklab/app/Chrome/Model/ViewPanes.cs`;
  - `tools/looklab/app/Chrome/Ui/ChromeFrame.cs`;
  - `renderer/shell/project/Shell.cs`, `CoreSurface.cs`, `D02/ShellJumble.cs`;
  - `renderer/shell/README.md`;
  - new `tools/looklab/tests/D02PaneTests.cs`;
  - `tests/test_renderer_shell.py`.
- **Add:**
  - `ViewPanes`, pure:
    - `Layout(openings)` gives the surface rectangle (the openings' bounding box) and each pane's rectangle within it;
    - `MaxPanes = 4`;
    - `Split(view)` gives two side-by-side panes of one view (used by B20 and B21);
    - `Hit(point)` gives the view and the point in the surface, or nothing.
  - With `--d02`, Local's frame is an opening as Global's is.
  - The surface moves and resizes to the bounding box under today's six-step rule, through the existing `sa2_surface_move` and `sa2_resize` calls only.
  - The chrome between the openings is opaque.
  - Camera keys go to the focused view.
  - Link: Local's rotation centres the named cap's pole; with no cap it copies Global's.
  - A click resolves through `ViewPanes.Hit`. A reply naming another view is dropped.
  - The event log's `frame` row gains `panes`.
- **Check:**
  - Console tests: `D02PaneLayoutIsBoundingBox`; `D02PanesNeverOverlap`; `D02FifthPaneRefused`; `D02HitResolvesItsView`.
  - `python renderer/shell/d02_run.py --script two-views` on `wj-ia-300`:
    - both views name the same revision after each of ten twists and after an undo;
    - a click in each view selects the expected piece;
    - `M600_FRAME_PICTURE` is transparent inside both openings and opaque between them.
  - A `tests/test_renderer_shell.py` case: without `--d02` the launch environment and the shell's core calls are unchanged.
- **Owner sees:** `watch_d02.py --state wj-ia-300`: Global and Local drawn at once; a twist moves both.
- **Machine:** H, W.
- **Checkpoint:** no; C3 follows B19.
- **Depends:** B18b.

**Frame budget with two views.**
- *Observation:* a hidden slot is collapsed after its vertex is evaluated (`renderer/core/shaders/product.hlsl:20`), so each pane costs a full vertex pass. Two panes double `view_vertex_evals`; three triple it.
- *Observation:* the pose arrays are adopted once and shared. Each further view adds only its three per-slot buffers, and the depth and ID targets are the bounding box's size.
- *Inference:* fragment work does not grow with the pane count, because the panes share one target.
- *Inference:* one bare view measured about 610 to 670 fps at a p99 of about 1.9 ms with 217 MB of VRAM (`docs/wiki/decisions/renderer-freeze.md:58` to `:61`). Three passes fit 33.3 ms on paper.
- That is not evidence for this route under the full chrome with styles and lines. G9 to G11 decide.

### B19. Gate sitting 1
- **Goal:** gate rows for drawing part 1, on the owner's machine, with the full chrome.
- **Items:** none; it gives R5 its numbers.
- **Files (new):** `tools/perf/d02/gate_row.py`, `renderer/shell/gate_d02.py`, and a results page under `docs/progress/1.0/`.
- **Add:**
  - `gate_d02.py --row <id>` launches with a script and the event log.
  - `gate_row.py` reads `events.jsonl`'s `presented` rows and prints the average rate and the 99th-percentile interval.
  - Rows. With `--d02` the default is two drawn views, so every row runs with both unless it says otherwise:

    | Row | State | Views and script |
    |---|---|---|
    | G1 | `deep-2` | Local hidden, Global with the contact-class tint, twist sweeps repeating for 60 s: the one-view baseline |
    | G9 | `deep-2` | G1's script with Local shown: the second view's cost |
    | G2 | `wj-ia-end` | two views, a blocked cap named, with all line layers |
    | G3 | the one-blocked-grip state the record names (section 6) | two views, certificate glyphs, at least 1,654 points |
- **Check:** each row prints `D02_GATE row=<id> panes=<n> view_vertex_evals=<v> fps_avg=<x> p99_ms=<y> vram_gb=<z> pass=<bool>` against 30 fps, 33.3 ms and about 7 GB. A failing row is recorded and goes to the owner; it is never tuned away. A failing G9 goes to the owner.
- **Owner sees:** the four runs, and the certificate glyphs for the owner's acceptance.
- **Machine:** G. About 35 minutes with the owner at the start.
- **Checkpoint:** yes, C3.
- **Depends:** B15 to B18c.

### B20. The cut slice in Local
- **Goal:** the slice as Local's overlay, with the held split as two linked copies.
- **Items:** .17.
- **Files:** core `shaders/product.hlsl` and `src/product.cpp`; new `Model/JumbleViews.cs`; `ShellJumble.cs`; new `tests/D02ViewTests.cs`.
- **Add:**
  - Slice mode in Local's style slot:
    - the outside layer filled;
    - the inside layer, from the named cap's inside mask, as outlines at a smaller shrink;
    - a radius-shell clip about the pole.
  - Global keeps its chain or chart.
  - The held key calls `ViewPanes.Split(Local)`: two panes with the same camera matrix, scale and shell. The left has the outside layer filled; the right has the inside layer in outlines. Release returns to one pane.
  - The turn preview is a ghost of the inside layer with `pattern.preview`; the twist is named on release.
  - At a blocked cap: no ghost, only the straddler sections with their point pairs.
- **Check:**
  - `D02SliceNamesTwistOnReleaseOnly`; `D02BlockedCapHasNoGhost`; `D02SplitCopiesShareRotationScaleShell`.
  - `check_core.py --slice` on WARP: outside pieces filled and inside pieces outline only; the split panes' pieces together equal the unsplit frame's.
  - `d02_run.py --script slice`: three panes while the key is held, two after.
- **Owner sees:** Global whole and Local sliced, side by side; hold the key and Local parts into two linked copies.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B16, B18, B18c. CR-14.

### B21. The raw view
- **Goal:** `view.mode.raw` in Local, with the antipode chart side by side.
- **Items:** .18.
- **Files:** core (`shaders/product.hlsl` variant, `src/product.cpp`, `src/selftest.cpp`); host `src/lib.rs`; `Model/JumbleViews.cs`; `ShellJumble.cs`.
- **Add:**
  - A precompiled stereographic variant from the named pole, declared through `c600_display::chart::Stereographic`.
  - The half-space clip x·p̂ ≥ h, default h = (121/125)·inradius, never past 90°.
  - The static tick for the meeting poles.
  - One key adds the antipode: `ViewPanes.Split(Local)`, with the right pane the pole chart after −I.
    - It shares the clip and the handedness, with a determinant sign checked at build.
    - It is labelled "antipode".
    - At h = 0 each sticker goes to one pane by its anchor, half-open.
  - The caption "No raw view certifies no holes; N = 0 does."
- **Check:**
  - `check_core.py --raw` on WARP:
    - a twist of the named cap is a rigid rotation about the chart centre in both panes;
    - nothing draws past the clip;
    - at h = 0 each sticker draws in exactly one pane.
  - `D02RawCaptionAlwaysShown`; `D02AntipodeNeverSaysMirrored`; `D02AntipodeSharesClipAndHandedness`.
  - `d02_run.py --script raw`: three panes with the antipode on.
- **Owner sees:** the named cap's region flat beside Global; one key adds the antipode chart next to it.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B20. CR-14. Holes drawn as faces wait for the exposed-face geometry.

### B22. Global chart preset: the ring sheet
- **Goal:** `view.preset.global.ring_sheet`, with no sweep under a chart.
- **Items:** .9, .10.
- **Files:** host `src/lib.rs` (chart assignment), core (per-piece chart index and 600 chart transforms), `Model/JumbleViews.cs`, `ShellJumble.cs`.
- **Add:**
  - Per revision, on the CPU in the host, for Global only: each posed sticker's nearest cell, ties to the lowest cell id.
  - 20 rows of 30 rigid cell charts from B8's layout, cyclic columns with a wrap mark, the caption "only row neighbours are face neighbours". The per-piece chart index and the 600 chart transforms belong to Global's style slot; Local is untouched and keeps its chain, slice or raw mode.
  - ADR-D02.10 is built whole: under a chart Global draws no sweep, and at a new revision the moved set is outlined for the half-turn row's time; Local sweeps a twist of the named cap; with the slice in Local, a twist of another cap is swept by no view.
  - The palette's chart row applies the ring sheet first; the projection chain stays the default until R5 admits a chart (.8).
- **Check:** host test `the_chart_assignment_is_nearest_cell_lowest_id`. `check_core.py --chart` on WARP: every piece draws inside its assigned chart's rectangle. `d02_run.py --script ring-sheet`:
  - a named-cap twist fires the sweep row in Local only;
  - a twist of another cap with the slice in Local fires none;
  - a click in a chart selects the piece its chart shows.
- **Owner sees:** the deep export as 20 rows of 30 cell charts with tints in Global, while Local turns the named cap.
- **Machine:** H, W. Record the assignment's time per revision.
- **Checkpoint:** no.
- **Depends:** B8, B15, B18c. CR-14.

### B23. Global chart preset: the Hopf rings
- **Goal:** `view.preset.global.hopf_rings`.
- **Items:** none new (.9's second preset).
- **Files:** as B22.
- **Add:** 20 circles of 30 cells around a dodecahedron's vertices, turning with the camera, with the same assignment and the same no-sweep rule. The palette switches between the two presets at any time.
- **Check:** `check_core.py --chart hopf` on WARP: the 20 ring centres project to the dodecahedron's vertices. `d02_run.py --script hopf`: switching presets changes neither a state digest nor Local's panes.
- **Owner sees:** the same state as rings; switch back and forth.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B22.

### B24. Gate sitting 2
- **Goal:** gate rows for the slice, the raw view, both presets and the three-pane cases, plus the occlusion measure.
- **Items:** none.
- **Files:** `renderer/shell/gate_d02.py`, the results page.
- **Add:**
  - Rows:

    | Row | State | Views and script |
    |---|---|---|
    | G4 | `deep-2` | Global chain, Local slice, twists of the named cap |
    | G5 | `deep-2` | Global chain, Local raw |
    | G6 | `deep-2` | Global ring sheet, Local chain carrying the sweeps |
    | G7 | `deep-2` | Global Hopf rings, Local chain |
    | G10 | `deep-2` | Global chain, Local slice with the split held for 60 s: three panes |
    | G11 | `deep-2` | Global ring sheet, Local raw with the antipode: three panes, the heaviest case |
    | G8 | `deep-2` | the simulation matrix held (B14's view style), on G9's two views |
  - The occlusion of both presets at the default camera, from a pick-pass count.
- **Check:** the `D02_GATE` lines, as in B19. A failing three-pane row withholds only the split or the antipode.
- **Owner sees:** the seven runs.
- **Machine:** G. About 45 minutes with the owner at the start.
- **Checkpoint:** yes, C4. After it the integrator can put R5 to the owner with both sittings' numbers.
- **Depends:** B20 to B23.

### B25. Word flows and signatures
- **Goal:** the word line, counters, the folded tree, signatures.
- **Items:** .29, .31.
- **Files:** new `Model/JumbleWords.cs`, `Ui/DraftLine.cs`, `Ui/LibraryPanel.cs`, `Model/Library.cs`, `tests/D02WordTests.cs`.
- **Add:**
  - `WordTree(Name, ExpandedLength, Parts)`.
  - The counter "part j of p, twist k of n", with playback by part, then by twist.
  - `LongWordBound`, a labelled placeholder until X3.
  - The crossing sentence, shown only with reachability.
  - Signatures: 35 slots and the ν row, or the undefined numeral with the length and alphabet.
  - The log tab's ticks at N = 0.
- **Check:** `D02WordLengthIsExpanded`; `D02PlaybackStepsByPartThenTwist`; `D02OffLatticeWordSignatureUndefined`; `D02CrossingSentenceNeedsReachability`. Then `d02_run.py --script seed7001`: the nine words played on the seed 7001 fixture show each word file's digests.
- **Owner sees:** the seed 7001 restoration played part by part, to solved.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B6.

### B26. The ν band and ν-corrections
- **Goal:** the band beside the rail at N = 0, and the Built-in branch.
- **Items:** .28, .39.
- **Files:** new `Model/NuBand.cs`, `Ui/NuBand.cs`, `Ui/OrbitRail.cs`, `Model/Library.cs`, `Ui/LibraryPanel.cs`, `tests/D02NuTests.cs`.
- **Add:**
  - 35 cells aligned with the rail's cells; the 8 frame-group marks; O17 and O28 as five-position dials; nine fixed dots; a mark shape outside the ring family; the undefined numeral off the lattice.
  - 36 correction rows in id order. Shift+Enter previews one; Space marks a signed multiplicity; one chord checks the set and shows "ν after this set" and "labels not predicted".
- **Check:** `D02NuBandAlignsWith35Cells`; `D02C5ValuesAreDialsNotBars`; `D02NuBandUndefinedOffLattice`; `D02CorrectionsAre36InIdOrder`; `D02CorrectionSetCheckMatchesSample`.
- **Owner sees:** on a ν-correction state, the band with marks and dials; mark two rows and check the set.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B6.

### B27. Keys, terms and the remaining unavailable states
- **Goal:** the held seat keys, the glossary check, and a named state for what still waits.
- **Items:** .8, .42, .43.
- **Files:** `Model/KeyMap.cs`, `Ui/KeyMapPanel.cs`, `tools/looklab/data/design/keymap.json`, `Model/JumbleViews.cs`, new `tests/D02KeyTests.cs`, new `tests/test_d02_words.py`.
- **Add:**
  - Four held seat entries with placeholder keys and glyphs. `KeyMapModel.HeldSeat(seat)` shows its 12 twist keys with their rotation orders.
  - Global's border names its chain: "projection chain (600-cell-Full)" by default.
  - "exposed faces unavailable" on the border and in Tier 1, with `input.exposed_geometry_unavailable`.
  - `test_d02_words.py`: every term of art in D02's words is a glossary row or one of the eleven requested additions in `PENDING_TERMS`.
- **Check:** `D02HeldSeatShowsTwelveTwistsWithOrders`; `D02NoDuplicateChordAmongD02Bindings`; `D02GlobalNamesItsChain`; `D02ExposedFacesUnavailableOnBorderAndTier1`; `python tests/test_d02_words.py`; `python docs/progress/1.0/glossary/build.py --check` unchanged.
- **Owner sees:** hold a seat key in the key map; its twelve twist keys light.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B2. CR-19 for the route; XM1 for the bindings; the glossary request.

### B28. The research pane: Records and Claims
- **Goal:** `pane.research` in Analyse, with its verdict words.
- **Items:** .3, .44, .45, .46, .49.
- **Files:** new `Model/Research.cs`, `Ui/ResearchPane.cs`, `renderer/shell/project/D02/ShellResearch.cs`, new `tools/looklab/data/samples/research/panel.json`, `tools/looklab/data/design/workspaces.json`, new `tests/D02ResearchTests.cs`, new `tests/test_d02_research_sample.py`.
- **Add:**
  - `IResearchSource`, with one method per existing research row.
  - Four tabs; virtual lists grouped by stage, then family.
  - A record opens in place with its finding and a folded tree of named parts.
  - A researcher's name is its own field and changes no digest.
  - `VerdictLine(verdict, scope)`: the exact sentences of ADR-D02.46, one weight, no colour role.
  - "pinned: input <digest>" never goes stale on a commit. The branch is read-only.
  - Copy and save set `Origin.Research` and carry their note.
  - The sample is synthetic, with field names from `rust/c600-research/src/panel/answers.rs` and `wire.rs`, labelled `evidence: "synthetic"`.
- **Check:** `D02VerdictWordsExact`; `D02NotApplicableCountsAsNoCheck`; `D02CutOffIsNeverAVerdict`; `D02PinnedInputNeverStaleOnCommit`; `D02RenameChangesNoDigest`; `D02EveryResearchControlInvokesARow`; `python tests/test_d02_research_sample.py`.
- **Owner sees:** the Research pane in Analyse; a Fail row expanded to its first failing claim.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B13.

### B29. The research pane: Runs and Obligations
- **Goal:** declared budgets, reproduction, obligations, the two CR-17 controls.
- **Items:** .4, .50.
- **Files:** `Model/Research.cs`, `Ui/ResearchPane.cs`, the sample, `tests/D02ResearchTests.cs`.
- **Add:**
  - `RunBudget`, declared before start. A run at its budget is `BudgetStopped`.
  - A seed field accepting 7401 to 7499 and naming the scored blocks; a depth field.
  - Reproduce, with its two sentences.
  - The Obligations tab.
  - "Open derived state in a free view" and "step the checker": both unavailable with the reason "row not in the command table yet (CR-17)". The first is also refused past the four-view envelope.
  - History shows no research runs.
- **Check:** `D02RunNeedsDeclaredBudget`; `D02RunAtBudgetIsCutOffNotVerdict`; `D02SeedOutsidePoolRefused`; `D02HistoryShowsNoResearchRuns`; `D02Cr17ControlsUnavailableWithReason`. Then `d02_run.py --script research`.
- **Owner sees:** declare a budget, start the sample run, see it cut off, reproduce it.
- **Machine:** H, W.
- **Checkpoint:** yes, C5.
- **Depends:** B28. CR-17.

### B30. The tuning panel: knobs, ramps, compare
- **Goal:** `panel.tuning` in the right dock, with live ramps and two slots.
- **Items:** none (first half of .47).
- **Files:** new `Model/Tuning.cs`, `Ui/TuningPanel.cs`, `ShellJumble.cs`, `tests/D02TuningTests.cs`.
- **Add:**
  - Family; Look; the five seed knobs over live ramp strips; Compare, with two slots, Swap and a held key.
  - The palette opens it as a tab in the current layout's right dock.
  - A knob change also sends the new palette to the core through `m6h_set_view_style`, so the pieces follow.
- **Check:** `D02TuningOpensInRightDockOfCurrentLayout`; `D02KnobChangeTouchesNoState`; `D02SwapIsAnInvolution`; `D02HeldCompareRestoresOnRelease`. Then `d02_run.py --script tuning`: the state digest is unchanged after every knob.
- **Owner sees:** move a knob; the frame and the tinted pieces follow.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B15. CR-16.

### B31. The tuning panel: presets, Advanced, simulation, Save checks
- **Goal:** complete ADR-D02.47.
- **Items:** .47.
- **Files:** `Model/Tuning.cs`, `Ui/TuningPanel.cs`, `Model/Gallery.cs` (`ColourMetric`), new `tools/looklab/data/design/tuning-presets.json`.
- **Add:**
  - Shipped presets are read-only and fork on edit. Advanced sits behind a switch, with search.
  - The held colour-vision and high-contrast simulation runs over the real frame: the same 3×3 matrix goes to the core's view style (B14) and to a chrome filter.
  - `Save()` fails in place, naming the failing pair, on the contrast rules, a non-monotone height ramp, or contact-class colours not pairwise distinct under each G4 simulation.
- **Check:** `D02SaveFailsOnNonMonotoneHeightRamp`; `D02SaveFailsOnCollidingClassesUnderSimulation`; `D02SaveFailsOnContrastRule`; `D02ShippedPresetForksOnEdit`; both themes pass `Save()` as shipped.
- **Owner sees:** hold the simulation key and both the pieces and the chrome change; a bad ramp is refused with its pair.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B30. CR-16.

### B32. The reduced Track J driver: J1
- **Goal:** a model-free run in the product shell, through keys and the palette only.
- **Items:** none (the harness for .56).
- **Files:** new `renderer/shell/track_j.py`, `tools/looklab/data/samples/jumble/j1-session.json`.
- **Add:** the session runs through `d02_run.py`'s player on the `wj-ia-300` and `wj-ia-end` fixtures. Section 7 gives its four parts.
- **Check:** `python renderer/shell/track_j.py --reduced --only j1 --out <new folder>` prints `M600_D02_J1_PASS operations=<k> reached=<k> boundary=0 observation=<n>/<n> not_run=<list>`.
- **Owner sees:** `watch_d02.py --script j1`: the same session at human speed.
- **Machine:** W.
- **Checkpoint:** no.
- **Depends:** B1 to B31.

### B33. J0, the researcher subagent, and the report
- **Goal:** planted defects, the seed 7001 replay, the lean subagent pass, one report.
- **Items:** .56.
- **Files:** `renderer/shell/track_j.py`, `Model/Jumble.cs` (a fault switch in `SampleJumbleSource`, read from `M600_D02_FAULT`, refused without `--d02`), new `tools/looklab/data/samples/jumble/defects.json`, a results page under `docs/progress/1.0/`.
- **Add:** at least 20 defects, each `{id, change, rule broken}`; the driver's loop over them; the subagent pass of section 7.
- **Check:** `python renderer/shell/track_j.py --reduced --out <new folder>` prints `M600_D02_J0_PASS planted=<n> caught_driver=<a> caught_subagent=<b> caught_any=<m> replay=9/9`, with m/n ≥ 0.8.
- **Owner sees:** the J1 session, then the two result lines.
- **Machine:** W.
- **Checkpoint:** yes, C6.
- **Depends:** B32. The ten sealed defects from C5.

## 3. Checkpoints

Each checkpoint is one read of the named diff range against the questions below, about an hour, plus what the owner is shown. Only blockers and majors stop the next block.

**C1, after B6.**
- Fable checks:
  - the sample digests equal section 6;
  - a sample binds to a session only by its pose-array digests;
  - `ToolSlot` satisfies J-7 and N-1;
  - the undefined numeral switches on the lattice flag only;
  - the entry slot is chrome only;
  - nothing D02 runs without `--d02`;
  - no protected file is in the diff.
- Owner is shown: the W-J state posed under the frame, with the Jumble cell and marks.

**C2, after B13.**
- Fable checks:
  - the Q3.b test compares the whole screen;
  - no path orders, weights, pre-focuses or pre-stages a repair row;
  - no call to `Stage` lacks a user event;
  - blocked caps are undefined everywhere;
  - total N is never derived from caps;
  - research origin is refused at admission.
- Owner is shown: the cap map, the lists, a staged repair.

**C3, after B19.**
- Fable checks:
  - an empty style or line layer draws byte-identically to none;
  - style and transform are bound to their revision;
  - the class rule is proper and sticky on the real samples;
  - the never-hidden set survives every focus layer;
  - a sweep draws only the certified inside set;
  - the gate lines come from the event log and not from an estimate;
  - a one-pane frame is byte-identical to today's frame, and runs without `--d02` take the one-view path;
  - no frame mixes revisions, and a hidden Local adds no vertex evaluations;
  - a pick reply comes from the view clicked;
  - G9's line comes from the event log.
- Owner is shown: gate sitting 1, Global and Local drawn at once, and the certificate glyphs to accept or refuse. About 90 minutes of Fable's reading.

**C4, after B24.**
- Fable checks:
  - the split copies share rotation, scale and shell;
  - the antipode shares the clip and the handedness and never says "mirrored";
  - the chart assignment is nearest cell with the lowest-id tie;
  - under a chart only Local sweeps, and with the slice in Local no view sweeps another cap;
  - the raw view's clip never passes 90° and its caption always shows;
  - the pane count never exceeds four;
  - no view mode changes a state digest;
  - the three-pane gate lines come from the event log.
- Owner is shown: gate sitting 2, both presets beside a drawn Local, the split and the antipode.

**C5, after B29.**
- Fable checks:
  - the verdict words are exact, and cut-off, not-checked and NotApplicable are never a pass or a fail;
  - every research control invokes a row;
  - the synthetic sample is labelled.
- Fable also writes ten planted defects, sealed, added to `defects.json` only after B32's driver is committed.
- Owner is shown: the research pane.

**C6, after B33.**
- Fable checks:
  - caught defects are counted honestly;
  - rules not run are reported as not run;
  - the subagent stayed inside its cap;
  - section 4 still matches the code.
- Owner is shown: the J1 session and the report.

## 4. Item map

| Item | Mapping | Note |
|---|---|---|
| .1 | B2 | Reach is tested again in B32 |
| .2 | B6 | |
| .3 | B28 | |
| .4 | B29 | |
| .5 | B13 | |
| .6 | B6 | |
| .7 | B3 | |
| .8 | B27 | Stays the default until R5 admits a chart |
| .9 | B22 | Hopf rings in B23; ships only after R5 and each preset's gate row |
| .10 | B22 | Local carries the motion (B18, B18b) |
| .11 | B6 | |
| .12 | B6 | The armed mode and the never-hidden set draw from B15 and B16 |
| .13 | B8 | After M-RING is redone in the step |
| .14 | B8 | |
| .15 | B9 | |
| .16 | B9 | |
| .17 | B20 | The held split is two linked copies (three panes) |
| .18 | B21 | The antipode sits side by side; holes as faces wait for .25 |
| .19 | B9 | Sticker picks in B17; glyph picks after the glyph gate row |
| .20 | B15 | |
| .21 | B15 | |
| .22 | B7 | |
| .23 | B15 | |
| .24 | B15 | |
| .25 | waits for the exposed-face geometry | Nothing in the hole; "exposed faces unavailable" on the border and Tier 1 (B27); slots exist (B2) |
| .26 | B16 | Glyphs stay a count until gate row G3 and the owner's acceptance |
| .27 | B2 | Drawn in B8 and B9 |
| .28 | B26 | |
| .29 | B25 | |
| .30 | B15 | Layer model in B11 |
| .31 | B25 | X3's bound is a labelled placeholder |
| .32 | B12 | |
| .33 | B13 | Engine write waits for CR-15 and K2 |
| .34 | B12 | The outline is drawn in B16 |
| .35 | B10 | |
| .36 | B10 | |
| .37 | B2 | Applied in B18 and B22 |
| .38 | B11 | |
| .39 | B26 | |
| .40 | B3 | |
| .41 | B2 | |
| .42 | B27 | Routing waits for CR-19; bindings for XM1 |
| .43 | B27 | |
| .44 | B28 | |
| .45 | B28 | Its two CR-17 controls are B29; the four-view refusal reads `ViewPanes.MaxPanes` |
| .46 | B28 | |
| .47 | B31 | First half in B30 |
| .48 | record only | Round 4 |
| .49 | B28 | |
| .50 | B29 | |
| .51 | record only | The reduced run's subagent is not this agent |
| .52 | record only | B29's seed field enforces the pool |
| .53 | record only | |
| .54 | record only | Run by B32 and B33 |
| .55 | record only | |
| .56 | B33 | |

## 5. Rows and data

**Missing command rows.** Each is a draft row in `tools/looklab/data/design/command-rows-d02.json`, registered by the shell only with `--d02`, and deleted when `D02DraftRowsAbsentFromTable` fails because the table gained the id.

| Request | Steps | Built before the row exists | Waits |
|---|---|---|---|
| CR-14 | B6, B9, B11, B12, B15, B20 to B23, B26 | Every view-kind operation works through its draft row | The product rows |
| CR-15 | B13 | Sealed span, whole-only bar, the record in the sample preview | The engine's preview and record write (K2) |
| CR-16 | B30, B31 | The whole panel | The product rows |
| CR-17 | B29 | Both controls shown as unavailable with the reason | The debugger trace and the derived view |
| CR-19 | B27 | The held-seat display | Routing a held seat with a twist key as one chord |
| Glossary | B27 | `PENDING_TERMS` | Emptied when LP4's file has the additions |

**Renderer and host work, by feature-matrix row.** Each is built as a candidate behind `--d02` and measured. Admission is R5, a revision of the frozen render chapters that the top integrator writes and the owner signs; no step edits those chapters.

| Work | Matrix row | Steps | Gate row |
|---|---|---|---|
| Posed pieces at sample states | J02 (exists) | B5 | G1 |
| Per-piece style: tint, outline, dim | J07, J09 (proposed additions) | B14, B15 | G1 |
| Lines: cut trace, outlines, certificate glyphs | J04 | B16 | G2, G3 |
| Picks | J13 | B17 | none |
| Sweep and half-turn | J03 | B18 | G1 |
| Two views, composite frame, picks per view | F27 | B18a to B18c | G9, G10, G11 |
| Slice layers and shell clip | J04, J09 | B20 | G4 |
| Stereographic stage and clip | J05 | B21 | G5 |
| Chart presets | J05 and the chart admission clauses | B22, B23 | G6, G7 |
| Exposed faces | J10 | none | waits: geometry production is owed by the engineering line |

**Other data.**

| Data | Steps | Built before it exists |
|---|---|---|
| The four-view envelope's measured values (the look-cost revision) | B18a to B18c, B20, B21 | Global and Local drawn at once, plus a third pane for the split or the antipode, as candidates behind `--d02`; a fourth pane (a preview view, or the derived free view of CR-17) is not built; a fifth is refused |
| X1, X4, X5 | B3, B10 | A tick clock; `slot.fill` stays off |
| X3 | B25 | A labelled placeholder bound |
| X6 | B15 | The straddler layer is never a default |
| XM1 | B27 | Placeholder bindings |
| M-RING | B8 | Redone as a tracked test |
| M-FOLD | B21 | Redone as B21's WARP check of the chart's folding at the clip |
| A sweep while engine jobs run in the runtime child | none | Waits for K2 |

L1 is retired by the owner's answer: the product core's pose path replaces it.

## 6. Sample data

All states are written by `make_d02_samples.py` through `RustJumbleEngine`. Each also gets a session fixture (B5), bound to its sample by the two pose-array digests. Menu identity `fddc8532…958c` (I_a).

| State id | Source | Must match | Used by |
|---|---|---|---|
| `on-zero` | `start(I_a)`, no twist | The adapter digest, pinned at B1 and expected to equal the W-J fixture's `stages.start.digest` `e3b0c442…b855` | B6, B7 |
| `wj-ia-300` | `research/jumbling/fixtures/wj-I_a.json`, journal records 0 to 299 | Digest pinned at B1 from two equal runs; contact classes exactly 4 | B5 to B18, B32 |
| `wj-ia-420` | The journal to record 420 | Off-lattice 114,930; `stages.mid.digest` `940367a8b97ca199…` if B1's case holds | Anchor |
| `wj-ia-end` | All 841 records | Off-lattice 121,184; 27 admissible and 573 blocked; `stages.end.digest` `6904a731ad8caa88…` if B1's case holds | B8, B9, B11, B16, B19 |
| `ia-40` | `stop40_reference()` in `research/jumbling/differential/references/obs_refs.py:125` (the W-J I_a fixture at 40 records) | Contact classes exactly 1 | B15 |
| `deep-1` | `research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json`, the 3,000-twist `history` | `1335f47e0c5647a948dd11f87ba4bad27c787d00992be1f783d970c88a542ace`; N 145,574; off 118,821; heights 116,665 / 1,971 / 185 | B7, B15 |
| `deep-2` | `deep-1`, the record's 313 `moves`, then the 12 letters at `research/jumbling/differential/leads/results/summary.md:497` | `bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`; off 103,896; 102,960 at height 2; 936 at height 4 | B7, B15, B19, B24 |
| `s7001-0` to `s7001-9` | `…/11-scramble-census/runs/round1/ia-k10-s7001.json` `history`, then `…/13-tutorial-runner/runs/depth10-seed7001/words/00000.json` to `00008.json` | Each word file's `input_digest` and `output_digest`; N 13,270 and off 26,519 at the start; solved at the end | B25, B33 |
| `nu-c<id>` (two ids) | `on-zero` plus one correction from `correction_lookup(id, twists=True)` | Digest pinned at B1; N = 0; ν equals the correction's row | B26 |
| `corrections.json` | `corrections()`, `nu_constants()` | 36 rows in id order | B25, B26 |
| `cap-layout.json` | `model()` poles against `tools/looklab/fixtures/hopf-rings.json` | A bijection onto 20 rows of 30 | B8, B22 |
| The G3 state | *Integrator:* S1, the witness state of `research/jumbling/witness.py`: one jumble twist (c, g) of 9.99987° from solved, with 54 blocked grips (`research/jumbling/viewer/README.md:271`; its certificate figures at `:420` to `:423`). B1 checks whether the adapter reaches it; if not, G3 uses the sample state whose blocked grip has the most certificate points, and the results page records that count beside 1,654 | At least 1,654 certificate points | B19 |
| `research/panel.json` | Synthetic, shapes from `rust/c600-research/src/panel/` | Field names present in the Rust source | B28, B29 |

Notes:
- The W-J journal was written under the A1 to A4 contract revision. Replay it through the conversion `tests/test_jumbling_rust_candidate.py` uses for its W-J cases.
- Masks are stored for two or three named caps per state, and for repair rows 1 to 9 only.

## 7. The reduced Track J run

**Base: the scripted driver.** It uses no model. `renderer/shell/track_j.py` runs the product shell through `d02_run.py`'s player: chords and palette text in, screen text out.

- `python renderer/shell/track_j.py --reduced --out <new folder>` runs both bars.
- `python renderer/shell/watch_d02.py --script j1` lets the owner watch it.

**J1.**
- *Keyboard reach:* every row id starting `jumble.`, `research.` or `evidence.` and every D02 draft row is invoked from a fresh shell by keys alone.
- *Observation:* one scripted check per rule of `jumbling-observation.md` that this build can show, now including the never-hidden set and the cut trace. A rule this build cannot show, such as exposed faces, is listed under `not_run`.
- *The repair list's boundary:* the rows equal the adapter's in order; no focus on open; nothing staged before a digit or Enter.
- *The boundary count:* `BoundaryLedger` reads zero.

**J0.**
- *Planted defects:* at least 20, switched by `M600_D02_FAULT`; ten written by Opus, ten by Fable sealed at C5. Classes:
  - repair rows reordered or pre-focused;
  - N_c at a blocked cap;
  - total N as a sum of caps;
  - pending shown as undefined;
  - a late result overwriting a newer one;
  - a result without coverage shown as ready;
  - a folded row with a numeral;
  - a class reused across a contact;
  - a blocked cap counted as a non-member;
  - a cut-off worded as "none";
  - NotApplicable counted as a pass;
  - research output admitted to a solving preview;
  - a digest changed by one character;
  - a ΔN sign flipped;
  - a style byte for the wrong piece.
- *Seed 7001:* the nine words play on the seed 7001 fixture. Here the host's own engine executes them, and the state after each word binds to the sample whose digests the word file names.

**The researcher subagent (owner's answer 2).** One subscription subagent on a smaller model; no API, no source access, one session.
- *It reads:* the palette's row list with display names, the key map sheet, `jumbling-observation.md`, and the screen dumps of runs.
- *It never sees:* the samples, the defect list, the code.
- *It does:*
  - reach: writes one key script that reaches every operation in a list of at most 30 display names, with one revision allowed after its first dump comes back;
  - defects: reads the dumps of 10 runs of a fixed session (8 faulted, 2 clean, unlabelled) and says for each whether a rule is broken and which.
- *Cap:* at most 2 scripts written, 12 dumps read, 90 minutes, one context with no continuation, about 2,000 words of output. Opus runs the scripts and hands back the dumps; the subagent runs nothing.
- *Scoring:* reach passes if its script reaches every listed operation. A defect is caught if the driver or the subagent names it; both counts are reported apart, and so are false alarms on the two clean runs.

**What it proves.**
- The surfaces show the adapter's facts faithfully over a real session at the same state.
- The host rules and the boundary hold in the chrome.
- Every operation has a keyboard route that someone other than its author can find from the product's own aids.
- The seed 7001 restoration replays through the product's engine to solved.

**What it does not prove.**
- That the aids' facts come from the product's engine: they come from samples until K2.
- J2 to J4, or the target of ADR-D02.55.
- The researcher agent of ADR-D02.51.
- Real key input: keys are injected inside the shell; real keys are seen only in the owner's sittings.
- Anything about the four-view envelope.
- Human evidence.

## 8. Risks and findings

The integrator's fixes to the ADR text for the first return's findings are on disk, and I have not re-listed them. What follows is the code this build stands on and what the new direction adds.

**Findings.**
- **F1 (major).** The chrome library has none of the revision 4 and 6 host primitives: a search of `tools/looklab/app` and `tools/looklab/data/design` finds no tool-slot state, undefined numeral, whole-only bar or pattern role, and `tools/looklab/app/Chrome/Model/Session.cs:11` has four preview states. B2 and B3 build them.
- **F2.** `FocusLayers` at `tools/looklab/app/Chrome/Model/Gallery.cs:157` is a fixed pick-layer list. B11 extends it.
- **F3.** The shell registers only the command table (`renderer/shell/project/ShellChrome.cs:124`) and stages no samples (`renderer/shell/build_shell.py:62` to `:64`). B4 fixes both.
- **F4 (major).** The shell draws one core view. `renderer/shell/README.md:207` says Local is "not drawn yet", and `:119` that the host opens one view. The slice and the raw view are Local modes, and the held split and the side-by-side antipode need two views at once. B18a to B18c draw both.
- **F5.** The session fixture is made only by pressing the router's fixed keys on one cap (`renderer/bridge/host/src/fixture.rs:14` to `:15`, `:33`), so no sample state can be opened. B5 adds a word fixture.
- **F6 (major).** The product pipeline has no style, line or stage input. `renderer/core/include/sa2_interop.h:533` to `:546` gives a preparation only visibility, interaction and instance ids, and `:565` to `:589` gives a frame only the camera matrix. `renderer/core/shaders/draw.hlsl:19` and `:31` colour a sticker by its label's cell.
- **F7 (major, process).** The rows this drawing needs are not admitted. `docs/architecture/1.0/language/renderer-feature-matrix.md:349` to `:354` mark J07 to J12 "Proposed addition under RM.27", `:347` leaves chart admission to its own clauses, and `:352` says J10's geometry production is still owed. That file is in the frozen language folder.
- **F8.** With Jumble on nothing is pickable (`renderer/shell/README.md:142` to `:143`). B17 binds J13.
- **F9.** Measured runs never pass `--frame` (`renderer/shell/README.md:196` to `:197`), yet the packet requires the gate to hold with the full chrome. D02's gate rows are a new run kind, and their numbers are not comparable with C9's bare-window runs.
- **F10.** A shell started from a background process receives no keys (`renderer/shell/README.md:216` to `:218`). Scripted runs therefore inject keys inside the shell and prove nothing about Windows input.
- **F11 (minor).** `renderer/shell/README.md:57` says the host wrapper refuses a version other than 2, while `renderer/bridge/host/src/lib.rs:106` is 7. B5 moves it to 8 and should correct the line.
- **F12.** Three digest functions may be in play: the adapter's record digest, the runner's (the W-J start and seed 7001 end digests are both the hash of empty input: `research/jumbling/restoration/13-tutorial-runner/runs/depth10-seed7001/check.jsonl:10`), and the host's head digest. B1 decides the first pair by one assertion. B5 binds by pose-array digests, which the render contract makes equal for equal states.
- **F13.** ADR-D02's data tags still name L1 for .9, .10 and .17, and `docs/progress/1.0/adr-d02-plan.md:102` lists "Jumble drawing in Look Lab" as waiting. The owner's answer retires both; the integrator should reword them.
- **F14.** The research sample must be synthetic: the only driver of the panel's real answers is `rust/c600-research/tests/panel.rs`, in an LP5 crate.

Findings of the two-view addition:
- **F15 (major).** Nothing below the chrome can draw two views today.
  - The host has one view id and opens only it (`renderer/bridge/host/src/lib.rs:161`, `:398`).
  - Its viewport must equal the surface's size (`renderer/bridge/host/src/lib.rs:825`; `renderer/core/include/sa2_interop.h:579`).
  - Every frame clears the whole target (`renderer/core/include/sa2_interop.h:742`), and every surface frame presents (`renderer/core/README.md:80`), on one visual (`renderer/core/README.md:79`).
  - Two `sa2_frame_submit` calls would therefore erase each other and take two presents. B18a's composite frame is the narrowest change.
- **F16 (major, process).** The four-view envelope has no measured value.
  - Its only sources are a request to the look-cost revision, "an envelope that counts up to four full-detail views" (`docs/architecture/1.0/design/adr/ADR-D01-rev4-solving-surface.md:164`), and ADR-D02's mention (`docs/architecture/1.0/design/adr/ADR-D02-jumble-research.md:350`).
  - The frozen rule makes a ceiling "the largest stated workload with a passing run of the unrelaxed gate" (`docs/architecture/1.0/language/renderer-feature-matrix.md:151` to `:157`).
  - Further views are "Required" with "No evidence yet" (`docs/architecture/1.0/language/renderer-feature-matrix.md:329`).
  - G9 to G11 would be the first such runs. A failing row lowers the pane ceiling and never moves the gate.
- **F17.** The gate's text names no view count, and its bars apply to the whole frame. The composite design keeps the reading unchanged (one present per frame), but the comparison with C9's bare runs stays invalid (F9).
- **F18.** Each pane costs a full vertex pass, so the split's two half-filled copies cost two (`renderer/core/shaders/product.hlsl:20`). This framework counts every pane as a view against the envelope. Compacting a pane's instance list would save the pass, but is not in this build.

**Risks.**
- **R1.** The renderer block is the long pole: C++, HLSL, three ABI moves (core 6, host 8 and 9) and the pins that follow them in the shell and the tests. A failed gate row stops a view from shipping, not the chrome.
- **R2.** A deep fixture may take minutes to open if a reopen replays every commit. B5 measures it. If it is slow, deep states are used only at gate sittings and checkpoints.
- **R3.** The chart assignment runs per revision over 177,120 pieces and 600 cells. B22 records its time.
- **R4.** New `Ui/` files must compile in Look Lab's project as well as the shell's. Run Look Lab's Godot lane once per checkpoint as a regression guard, never as acceptance.
- **R5.** Gate sittings need a quiet machine and the owner's start. Batching them into two keeps that to under an hour and a half.
- **R6.** The surface now lies under the chrome between the two openings. Any translucent chrome there shows the clear colour. B18c's picture check covers it, and the owner's eye confirms it at C3.

**Questions only the owner can answer.** Both were answered on 26 November 2026: two drawn views, Global and Local at once; and the drawing candidates are built and measured behind `--d02` before R5, with R5 put to the owner once after gate sitting 2.

## 9. Estimate

| Step | Hours | Step | Hours | Step | Hours |
|---|---|---|---|---|---|
| B1 | 7 | B13 | 4 | B23 | 5 |
| B2 | 4 | B14 | 10 | B24 | 4 |
| B3 | 5 | B15 | 6 | B25 | 4 |
| B4 | 6 | B16 | 8 | B26 | 5 |
| B5 | 8 | B17 | 5 | B27 | 4 |
| B6 | 5 | B18 | 7 | B28 | 6 |
| B7 | 3 | B18a | 9 | B29 | 4 |
| B8 | 5 | B18b | 8 | B30 | 5 |
| B9 | 5 | B18c | 7 | B31 | 5 |
| B10 | 5 | B19 | 5 | B32 | 5 |
| B11 | 4 | B20 | 8 | B33 | 6 |
| B12 | 5 | B21 | 9 | | |
| | | B22 | 9 | | |

- **Opus:** about 210 agent hours over 36 commits, of which about 100 are the drawing block (B14 to B24). Each step is one session; B14, B18a and B22 are the largest and may split at the core and host boundary.
- **Fable:** six checkpoints of about an hour (C3 about 90 minutes), plus half an hour at C5 for the sealed defects.
- **The researcher subagent:** one session, at most 90 minutes.
- **The owner:**
  - six checkpoint sittings of 10 to 15 minutes, about 1.25 hours;
  - two gate sittings of about 35 and 45 minutes at the start, then a quiet machine while the rows run;
  - one acceptance of the certificate glyphs;
  - watching between sittings is optional: `python renderer/shell/watch_d02.py`.

## 10. Integrator's check and dispositions (26 November 2026)

**Check against the packet and the owner's answers.**
- Every item ADR-D02.1 to .56 is mapped once (section 4). Every step names its items, files, check, owner view,
  machine, checkpoint and dependencies. The row-dependent steps are marked (section 5).
- The product shell is the build, watch and acceptance surface. Look Lab only compiles the chrome library's `Ui/`
  files, as a regression guard once per checkpoint (R4), never as acceptance.
- Jumble drawing is in this build: B14 to B24, by feature-matrix row and gate row (section 5). Each step carries its
  machine mark, and the two gate sittings need the owner's machine, a quiet machine and the owner's start.
- The researcher subagent has a cap (section 7): two scripts, twelve dumps, 90 minutes, one context, no source.
- The existing files and folders the steps cite exist on this branch, and the cited lines say what section 8 says
  they do. The deep-state counts match `research/jumbling/differential/leads/results/summary.md:102` and `:103`.
- No step edits ADR-D01, the command table, a frozen chapter, an LP4 or LP5 crate or `assets/manifest.json`. The
  renderer features are built as candidates behind `--d02` and admitted only through R5 (question 2).

**Dispositions of the findings.**
- **F1 to F6, F8, F10, F12, F14:** adopt; each is carried by the step that names it.
- **F7:** adopt. R5 is written by the top integrator and signed by the owner; question 2 asks whether the candidates
  may be built and measured first.
- **F9:** adopt. D02's gate rows are a new run kind with the full chrome; their results page says they are not
  comparable with C9's bare-window runs.
- **F11:** adopt; B5 corrects the README line with the version move.
- **F13:** adopt, done on this branch: ADR-D02.9, .12 and .17, the (tuned) tag and the "Data still owed" list no
  longer name Look Lab's L1, and the plan's row is reworded. The learning-level names L1, L2 and L6 in ADR-D02's
  glossary request are a different thing and stay.
- **ADR-D02.47's look-cost row:** B31 sends the simulation's matrix to the core, but ADR-D02.47 lets the simulation
  cover the pieces only once a core stage and its look-cost row exist. Row G8 measures the matrix that B14's view
  style already carries, at gate sitting 2. B31 builds the core half switched off and turns it on only if G8 passed.
- **The G3 state:** filled in section 6 (S1 of the witness, with a fallback).
- **The two owner questions** of section 8 went to the owner on one card with the recommended option first. The owner
  chose two drawn views and took building and measuring before R5. Fable then wrote the two-view addition, spliced in
  above: B18a to B18c, B20 and B21 rewritten without stand-ins, changes to B14, B16, B19, B22 to B24, C3 and C4,
  sections 4, 5, 8 and 9. Its cited files and lines exist on this branch.
- **F15 to F18:** adopt. F16 means G9 to G11 are the first measured values for the four-view envelope; a failing row
  lowers the pane ceiling and never moves the gate. **R6:** adopt; B18c's picture check and the owner's eye at C3.
- B1 waits for the owner's go.
