# ADR-D02 shell: build-direction framework

Status: written by Fable as a subscription subagent on 26 November 2026, at the owner's go of the same day, from the
packet of the ADR-D02 session (seven parts, effort high, read-only, about 10,000 words by the owner's limit). Owner
decision behind it: [25 November 2026](../../wiki/decisions/owner-decisions-2026-11-25.md), item 5 (Fable frames, Opus
builds one step per commit, Fable reviews at checkpoints; subscription, no API). The record it builds is
[ADR-D02](../../architecture/1.0/design/adr/ADR-D02-jumble-research.md); its `path:line` citations of the ADR text
refer to main `4202cbd9`, before this branch's fixes. The integrator's check and dispositions are in section 10. Apart
from section 10, the text is Fable's, with one source corrected in section 6 (marked) and a local path removed.

This is the framework only: Fable changed no file and ran no build, test, Look Lab or cargo command. Everything below
comes from reading tracked source, so each finding is a lead until a test confirms it. Every relative path is from the
repository root.

## 1. Overview

**Shape (23 steps, 5 checkpoints).**
1. B1 to B5, foundations: sample data from the Rust adapter, design data, host primitives, the Jumble host, staging into the product shell.
2. B6 to B12, the Solve side: totals, cap map, threshold and naming, kept twist checks, Cap covers and focus layers, repair list, staging and the boundary.
3. B13 to B17: words and signatures, the ν band and ν-corrections, encodings, the unavailable view states, keys and terms.
4. B18 and B19: the research pane.
5. B20 and B21: the tuning panel, last of the shell.
6. B22 and B23: the reduced Track J run (J1, then J0).

**Where each surface lives.**
- **Chrome library** (`tools/looklab/app/Chrome/`, namespace `Magic600.Chrome`): every ADR-D02 surface.
  - `Model/` is pure C#; the test project compiles it, so every rule gets a console test.
  - `Ui/` holds Godot controls and writes no colour (the token lint).
  - One new assembly point, `Ui/JumbleSurface.cs`, attaches all surfaces to a `ChromeFrame`. The lab and the shell both call it, so there is one copy.
- **Look Lab** (`tools/looklab/app/Live/`): `LabJumble.cs`, `LabResearch.cs` and `LabTuning.cs` wire the surfaces to sample data, live reload, the kept view, the tour and the sweep.
- **Product shell** (`renderer/shell/`): `build_shell.py` stages the chrome library, the design data and, new in B5, the sample data. `project/ShellChrome.cs` calls the same `JumbleSurface.Attach`.
- **Host library** (`renderer/bridge/host`): not touched. The seam is the interface `IJumbleSource`, with a matching `IResearchSource`. This build binds them to sample files; K2 binds them to `m6h_invoke`.

**The live window.**
- Opus gives the owner the command `python tools/looklab/check.py --run`. The owner starts it in a terminal of their own, because a window started from a background process does not get the foreground.
- JSON under `tools/looklab/data/` reloads in the same window within half a second. A refused file keeps the last good lab and shows a notice.
- C# changes need the owner to close and rerun `--run`, because it stages a source copy. Opus says so at each commit that changes C#. The run has a two-hour limit.
- Every step adds one tour stop named `D02 Bn: <title>`. The owner presses F9 to walk the stops.
- The file `tools/looklab/data/samples/jumble/lab.json` selects the state, named cap, session kind and result variant. Editing it changes the window without a restart.

**Conventions for every step.**
- Run `python tools/looklab/check.py` at every commit.
- Run `python tools/looklab/check.py --godot` when `Ui/` or `Live/` changed.
- Run `python tools/wiki/lint.py` when a design data file changed (an evidence claim may cite it).
- From B5 on, run `python tests/test_renderer_shell.py`.
- New console tests go in `tools/looklab/tests/D02*Tests.cs` and are registered by name in `tools/looklab/tests/Program.cs`.
- Freeze round 3 values are placeholders. Each new role's `meaning` starts with `placeholder (round 3):`. Icons are tier 1 glyphs.

## 2. Steps

### B1. Jumble sample data from the Rust adapter
- **Goal:** tracked sample files for every state the surfaces show, rebuilt only through the adapter, each with its digest.
- **Items:** none directly; it supports B4 to B23.
- **Files (new):**
  - `tools/looklab/fixtures/make_d02_samples.py`
  - `tools/looklab/data/samples/jumble/` (`states.json`, `lab.json`, `corrections.json`, one folder per state)
  - `tests/test_d02_samples.py`
- **Add, in the builder:**
  - `build_state(spec)` drives `RustJumbleEngine`: `start(menu)`, `identity()`, `word(ident, letters)` per segment, `record()`. It asserts `status == 'admissible'` and `applied == len(word)` on each segment.
  - `facts(engine, caps)` writes these files per state:
    - `totals.json` from `heights()`, `seam_facts('all', …)` and `nu()`;
    - `rim.json` from `rim_summary()`, 600 rows;
    - `contact.json`: domain IDs, piece counts and contact edges, off-grip contacts included;
    - `cap-<id>.json` per named cap, with `covers`, `repair_list` in three variants (completed, `max_pairs` cut-off, `cancel_after` cancelled), `release_check` for the twelve twists of one seat, `straddlers` and the certificate-point count.
  - Modes: `--write` and `--verify` (rebuild and compare byte for byte).
  - File format `magic600-d02-sample/1`. Every file carries `state`, `digest`, `menu_identity`, `pin` and `evidence: "source"`.
  - No per-piece arrays: nothing here draws pieces.
- **Decide first (finding F12):** assert that `start(I_a)` then `record()['digest']` equals the W-J fixture's `stages.start.digest`. If it differs, the adapter's digest is a different function from the runner's, and the seed 7001 chain must compare through `j1_from_record`. Record which case holds in `states.json`.
- **Check:**
  - `python tests/test_d02_samples.py` passes with no adapter. It checks the format, the pinned digests of section 6, that the rim classes sum to 600, the end-of-journal counts (27 admissible, 573 blocked) and the height counts of both deep exports.
  - `python tools/looklab/fixtures/make_d02_samples.py --verify` prints `D02_SAMPLES_PASS states=<n>` after the adapter build named in the packet.
- **Window:** nothing new yet.
- **Checkpoint:** no.
- **Depends:** the adapter build; about two minutes per deep state. If `seam_facts(all)` passes its budget at a deep export, store the cut-off result (it is the "incomplete" sample) and record the seconds as a lead.

### B2. Design data: slots, patterns, motion version 2, rows, words
- **Goal:** every slot name, pattern, motion row, draft command row and display word of ADR-D02 exists as data before any control uses it.
- **Items:** .1, .27, .37, .41.
- **Files:**
  - `tools/looklab/data/design/system.json` (roles) and `themes/graphite.json`, `themes/paper.json`
  - `tools/looklab/data/design/motion.json`
  - new `tools/looklab/data/design/command-rows-d02.json` and `jumble-words.json`
  - `command-labels.json`, `command-icons.json`
  - `tools/looklab/app/Live/LabFrame.cs` (one `registry.AddRows` call after line 121)
  - new `tools/looklab/tests/D02DataTests.cs`
- **Add:**
  - Roles, each mapped in both themes:
    - `numeral.undefined`, `commit.bar.whole_only`, `mark.practice`, `mark.jumble`;
    - `slot.tool.state.{empty, pending, ready, stale, refused, budget_stopped, error}`;
    - `pattern.cap.{blocked, closed, at_floor, above_floor}`, `pattern.cut.{inside_only, outside_only}`, `pattern.preview`;
    - `tint.jumble.contact_class.{1..6}`, `tint.jumble.contact_class.overflow`, `tint.jumble.height.{2, 4, 6, above}`;
    - `material.exposed_face`, `material.exposed_face.tinted`, `state.exposed_face.unavailable`.
  - `motion.json` at version 2 with `slice.turn`, `jumble.sweep`, `jumble.half_turn`, `jumble.enter`, `jumble.leave`, `repair.step`, `cap.name` and `slot.fill` (off, cue `cue.check` reserved; see F5). Each has `static: true` behaviour under reduced motion.
  - `command-rows-d02.json` with the draft rows of section 5. Each row has a `request` field (`CR-14` to `CR-17`, `CR-19` or `unlisted`).
  - `jumble-words.json` with every sentence the ADR quotes, keyed by id, and the display names of .41 with "Release check" as a synonym.
- **Check:** `python tools/looklab/check.py` with these new tests:
  - `D02RolesExistInBothThemes`;
  - `D02MotionRowsStaticUnderReducedMotion`;
  - `D02DraftRowsAbsentFromTable`: every draft id is missing from `command-table.json`, so the test fails on the day a row lands and the draft row must then be deleted;
  - `D02EveryDraftRowNamesItsRequest`;
  - `D02WordsMatchRecord`: byte equality with the quoted sentences.
- **Window:** the new rows appear in the palette (F4) with their labels. The token specimen in Gallery shows the new roles with placeholder values.
- **Checkpoint:** no.
- **Depends:** nothing.

### B3. Host primitives: tool slots, releases, numerals, the whole-only bar
- **Goal:** the revision 6 host rules ADR-D02 rests on exist in the chrome model. They are absent today (F1).
- **Items:** .7, .40.
- **Files (new):**
  - `tools/looklab/app/Chrome/Model/ToolSlots.cs`
  - `tools/looklab/app/Chrome/Ui/ToolSlotBox.cs`
  - `tools/looklab/tests/D02SlotTests.cs`
- **Add:**
  - `enum SlotState { Empty, Pending, Ready, Stale, Refused, BudgetStopped, Error }` and `enum JobEnd { Completed, Cancelled, CutOff, Error }`.
  - `record Coverage(int Cap, int Revision, string Pin, string Menu, string TieRule, int Tried, int Of)`.
  - `record ToolResult(JobEnd End, string InputDigest, string Parameters, Coverage? Coverage, string? Reason)`.
  - `class ToolSlot` with `Request(int release, string digest, string parameters)`, `Finish(int release, ToolResult)`, `Cancel(int release)`, `Invalidate(string digest)` and `Word(words)`. Its rules:
    - a result is taken only from the latest release, with matching digest and parameters;
    - a completed result with no coverage becomes `Error`;
    - cancelling the latest release restores the previous state;
    - any call for an older release changes nothing (N-1).
  - `class ReleaseRecord` with `Next()`, `TryNumber(cap, revision)` (restarting at each commit) and `Superseded(release)`.
  - `enum NumeralKind { Value, Undefined, Pending, Incomplete }` and `readonly record struct Numeral`.
  - `static class JumbleSlots` with the eight ids `slot.tool.jumble.{survey, straddlers, covers, release_check, repair, nu, corrections_check, checkpoint}`.
  - `ToolSlotBox`: one control that draws a slot state with its word and one non-colour mark, reusing `PanelStateBox` where it fits.
- **Check:** these tests pass under `check.py`:
  - `D02SlotLateResultIgnored`, `D02SlotCancelLatestRestores`, `D02SlotCancelOlderChangesNothing`;
  - `D02SlotCompletedWithoutCoverageIsError`;
  - `D02SlotWordsCoverSevenStates`, `D02SlotIdsAreTheEightNames`;
  - `D02NumeralKeepsUndefinedPendingIncompleteApart`.
- **Window:** a tour stop that cycles one demo slot through the seven states and their words.
- **Checkpoint:** no.
- **Depends:** B2.

### B4. The Jumble host in Solve
- **Goal:** Jumble on and off in the Solve layout on sample states, with no new layout or dock.
- **Items:** .2, .6, .11, .12.
- **Files:**
  - new `tools/looklab/app/Chrome/Model/Jumble.cs`
  - new `tools/looklab/app/Chrome/Ui/JumbleHost.cs` and `Ui/JumbleSurface.cs`
  - new `tools/looklab/app/Live/LabJumble.cs`
  - edits to `Ui/ChromeFrame.cs`, `Ui/SolvingSurface.cs`, `Ui/OrbitRail.cs`, `Live/LabFrame.cs`, `tools/looklab/lane_a5.py`
  - new `tools/looklab/tests/D02HostTests.cs`
- **Add:**
  - `interface IJumbleSource`, with one method per sample file, and `SampleJumbleSource.Load(folder, stateId)`.
  - `class JumbleState` with `Mode`, `LatticeFlag` (exact), `NamedCap`, `Revision` and `SessionKind { Solving, Research }`.
  - `class EntrySlot` for `view.preset.jumble_entry`. `Apply()` records what it set. `Revert()` undoes only what is unchanged since. It follows the adopted revision's mode.
  - `ChromeFrame.SetModeCell(Control)` and an enumeration of all view frames, so that `ViewFrame.AddMark` puts `mark.jumble` on every border.
  - The strip and the rail show `numeral.undefined` as a whole when `LatticeFlag` is false. A pending N never switches them.
  - Local's empty state, with its sentence from `jumble-words.json`.
  - A Local accordion with four headers (named-cap cell, kept twist checks, Cap covers, repair), one open at a time. Their bodies are `ToolSlotBox` for now.
  - `LabJumble.Handles(command)` for `jumble.on`, `jumble.off`, `view.preset.jumble_entry` and the lab-only row `lab.d02.sample`.
  - A kept-view part `frame.jumble`. Update `parts=15` in `tools/looklab/lane_a5.py` line 10 in the same commit.
- **Check:**
  - `D02EntryRevertsOnlyUnchanged`;
  - `D02EntryFollowsUndoRedoBranchReopen`;
  - `D02EntryIsChromeOnly`: it touches no projection, layout or shrink field;
  - `D02UndefinedSwitchesOnLatticeFlagNotPendingN`;
  - `D02MarkOnEveryViewBorder`;
  - `D02PanelsMoveFreely`: each D02 panel passes `FrameModel.Drop` to another dock and `Store`/`Apply` round-trips it;
  - `check.py --godot` passes with the new `parts` count.
- **Window:**
  - with the `on-zero` state: the Jumble cell, a mark on every border, true zeros, the empty Local sentence;
  - switching `lab.json` to `wj-ia-300` turns the strip and the rail to the undefined numeral.
- **Checkpoint:** no.
- **Depends:** B1 to B3. `view.preset.jumble_entry` is a draft row (CR-14).

### B5. Staging into the product shell
- **Goal:** the shell shows the same surfaces from the same code. From here every step is checked in both places.
- **Items:** none new.
- **Files:**
  - `renderer/shell/build_shell.py`
  - `renderer/shell/project/ShellChrome.cs`
  - `tests/test_renderer_shell.py`
  - `renderer/shell/README.md` (one section; also correct the ABI list, F4)
- **Add:**
  - In `build_shell.py`, a constant `SAMPLES = ROOT/tools/looklab/data/samples` and one `copytree` of its `jumble` and `research` folders to `project/samples`.
  - In `ShellChrome.cs`, after the registry at line 124: `registry.AddRows` for `command-rows-d02.json` and `registry.Relabel`, then `JumbleSurface.Attach(frame, registry, SampleJumbleSource.Load(…), words)`.
  - A row from the draft file shows a "draft row" mark in the palette.
  - The `Unavailable` switch keeps "not in this build yet" for everything else.
- **Check:**
  - `python tests/test_renderer_shell.py` with a new case `test_the_frame_stages_the_d02_surfaces_and_samples`: the build stages `samples/`, `ShellChrome.cs` calls `JumbleSurface.Attach`, and no second copy of a chrome file exists.
  - On the owner's machine at the checkpoint: `python renderer/shell/build_shell.py`, then `python renderer/shell/launch.py --out <new folder> --frame --puzzle <the Jumble-on value of PUZZLES>` shows the Jumble cell. This one is an observable result, not a headless check.
- **Window:** unchanged. The shell window is the second thing the owner sees at C1.
- **Checkpoint:** yes, C1.
- **Depends:** B4.

### B6. Totals and the named-cap readouts
- **Goal:** the numerals on Global's and Local's borders, with pending, incomplete and undefined kept apart.
- **Items:** .22.
- **Files:**
  - new `Model/JumbleReadouts.cs` and `Ui/JumbleReadouts.cs`
  - `Ui/JumbleSurface.cs`
  - new `tests/D02ReadoutTests.cs`
- **Add:**
  - `class GlobalTotals` with these fields as `Numeral`: N, its on-grip and off-grip parts, the domains, the off-lattice pieces, n2, n4, n6, and "above 6" with its largest height (`Undefined` at a count of 0).
  - `class CapReadout` with In, Out, A, N_c, floor and u, all `Undefined` at a blocked cap.
  - Fixed slots with tabular numerals, one neutral change mark, the unit word "seams". No gauge, and no colour by value.
- **Check:**
  - `D02TotalNPendingUntilSeamFactsAll`;
  - `D02TotalNIncompleteAfterCutOff`;
  - `D02TotalNIsNotSumOfCaps`: a planted sum fails;
  - `D02BlockedCapReadoutUndefined`;
  - `D02Above6MaxUndefinedAtZero`;
  - `D02TotalsMatchSample`: equality with `totals.json` for all states, including the deep-export counts of section 6.
- **Window:** the totals on Global's border for the deep export. Setting `"variant": "pending"` in `lab.json` shows the pending form.
- **Checkpoint:** no.
- **Depends:** B4.

### B7. The cap map
- **Goal:** one canvas control of 600 caps in the ring-sheet arrangement, in Solve's bottom dock.
- **Items:** .13, .14.
- **Files:**
  - new `Model/CapMap.cs` and `Ui/CapMapControl.cs`
  - new `tools/looklab/data/samples/jumble/cap-layout.json` (written by the B1 builder; add `layout()` to it)
  - `Live/LabJumble.cs`
  - new `tests/D02CapMapTests.cs`
- **Precondition (F6):** redo M-RING in the repository. The builder writes `cap-layout.json`: for each cap id, its row (ring) and column from `tools/looklab/fixtures/hopf-rings.json`, by matching the adapter's `model()` pole of each cap to the Look Lab cell it is the pole of. `test_d02_samples.py` checks three things: the map is a bijection onto 20 rows of 30; every row neighbour is a face neighbour; no other neighbour claim is made.
- **Add:**
  - `enum CapClass { Pending, Blocked, Closed, AtFloor, AboveFloor }`.
  - `CapMapModel.Cells`. The class comes only from certified, completed rim rows; anything else is `Pending`. N_c is a numeral at admissible caps only.
  - `CapMapControl`: one `Control._Draw`, with patterns from `pattern.cap.*`. Blocked is a dark ground with a non-slash glyph, closed is plain, at floor is a corner bracket, above floor is the numeral only.
  - A wrap mark on the cyclic columns, and the legend "at floor (N_c = F_c > 0)".
  - `frame.AddDockPanel(DockSide.Bottom, "capmap", …)`, collapsed by default. It can also be dropped as a free pane.
- **Check:**
  - `D02CapClassesExclusiveAndExhaustive`;
  - `D02NoNumeralAtBlockedCap`;
  - `D02UncertainRimRowIsPending`;
  - `D02CapMapMatchesRim`: 27 admissible and 573 blocked at `wj-ia-end`;
  - `D02CapLayoutIsRingSheet`;
  - `check.py --godot` (the sweep opens and closes the dock).
- **Window:** the bottom dock opens to 20 rows of 30 cells. `wj-ia-end` shows mostly blocked ground with 27 numerals.
- **Checkpoint:** no.
- **Depends:** B1, B4.

### B8. Threshold, naming a cap, targets
- **Goal:** the threshold predicates, naming by cell or by number, the 80 meeting caps, and the inspector tiers.
- **Items:** .15, .16, .19.
- **Files:** `Model/CapMap.cs`, `Ui/CapMapControl.cs`, `Ui/JumbleHost.cs`, `Live/LabJumble.cs`, `Live/LabInspect.cs`, new `tests/D02ThresholdTests.cs`.
- **Add:**
  - `record CapPredicate(Kind { AtLeast, AtMost, AtFloor }, int K)`.
  - `CapMapModel.Threshold`: a set with no default. `Counts()` returns members, non-members and blocked caps, counted apart; pending caps are counted as pending. Members are drawn as the Query set outline. There is no sort API at all.
  - The threshold field and its count sit on Global's border while the dock is closed.
  - `JumbleState.Name(cap)` by cell click, or by typing a number through `jumble.cap.name` (an unlisted draft row, F7).
  - The 80 meeting caps, from `cap-<id>.json`, shown as a static mark that is neither an outline nor a dim.
  - No slot leaves `Empty` before a cap is named. Hovering only inspects.
  - Tier 1 and Tier 2 facts for cap, piece and domain, in a fixed order, through `InspectorPanels`.
- **Check:**
  - `D02NoDefaultThreshold`;
  - `D02ThresholdCountsBlockedApart`;
  - `D02ThresholdNeverReorders`: the cell order equals the layout order under every predicate;
  - `D02NoJobBeforeNaming`;
  - `D02HoverNamesNothing`;
  - `D02MeetingCapsAre80`;
  - the keyboard-reach check passes with `pointer_only` still empty.
- **Window:** type a cap number. Its cell, the 80 marks and Local's readouts appear. Set `N_c ≥ 3` and read the three counts.
- **Checkpoint:** no.
- **Depends:** B7. CR-14 for `jumble.capmap.threshold.{set, clear}`; the draft rows work in the lab and the shell until the table has them.

### B9. Kept twist checks and the Q3.b test
- **Goal:** the release-ordered list with folded "not checked:" rows, under J-7 and N-1 as the owner reads them.
- **Items:** .35, .36.
- **Files:** new `Model/TwistChecks.cs`, `Ui/JumbleLists.cs`, `Live/LabJumble.cs`, new `tests/D02TwistCheckTests.cs`, new `tools/looklab/lane_d02.py` with one call from `tools/looklab/check.py --godot`.
- **Add:**
  - `class KeptChecks` with `Release(letter)`, `Complete(release, row)` and `Cancel(release)`.
  - `record CheckRow(int Try, string Twist, Numeral Nc, Numeral Floor, long DeltaN, …)` and `record FoldedRow(string[] Twists)`.
  - A folded row is written inside `Release`, from the release record alone. `Cancel` of a non-latest release is a no-op.
  - The session release number shows on hover.
  - A collapsed word-check row with its "lifted" field.
  - `class SampleJobs`: a deterministic clock that completes sample results after n ticks. The lab's release key feeds it from `cap-<id>.json`.
- **Check:**
  - `D02Q3bTwelveTwists` releases a seat's twelve twists faster than completion on the W-J sample and asserts the four points of ADR-D02.36:
    - completed rows are in increasing release order;
    - each folded row is born in the superseding release's frame;
    - a folded row carries only the words and the glyphs;
    - `Describe()` is equal before and after every cancel of a non-latest job.
  - The lane mode `d02-q3b` repeats the test on the real controls and prints `LOOKLAB_D02_Q3B_PASS releases=12 cancels=<n> screen_changes_on_cancel=0`.
- **Window:** hold the lab's seat key. Rows and folded rows build up, and nothing flickers when jobs cancel.
- **Checkpoint:** no.
- **Depends:** B3, B8.

### B10. Cap covers, straddlers and the predicate focus layers
- **Goal:** the Cap covers slot and the focus layers with counts.
- **Items:** .30, .38.
- **Files:** new `Model/CapCovers.cs`, `Model/Gallery.cs` (extend `FocusLayers`, F2), `Ui/JumbleLists.cs`, `Live/LabJumble.cs`, new `tests/D02CoversTests.cs`.
- **Add:**
  - `CapCoversModel`: the alignment-domain line, then "covered by", "covers" and "undefined", in cap-id order. The expanded table has 80 rows, both directions, each yes, no or undefined. Blocked meeting caps are listed as undefined.
  - `FocusLayers.AddPredicate(id, label, Func<int> count)`, joining the existing cycle on F10. Three predicate layers are added: straddlers of the named blocked cap, one domain, and height at least k.
  - The straddlers layer stays on across named caps and is active only at a blocked cap. `CapMapControl` ignores it.
- **Check:**
  - `D02CoversUndefinedIsListed`;
  - `D02CoversTableHas80RowsBothDirections`;
  - `D02HoverInCoversMarksNoCapMapCell`;
  - `D02StraddlerLayerNeverDefault`;
  - `D02StraddlerLayerOnlyAtBlockedCap`;
  - `D02FocusLayerCountsMatchSample`.
- **Window:** name a blocked cap and press F10 until "Straddlers (u = …)" shows with its count. The view itself carries the line "not drawn in this build: needs L1".
- **Checkpoint:** no.
- **Depends:** B8. CR-14 for the highlight-and-dim layer row. The dimming of pieces in a view waits for L1.

### B11. The repair list
- **Goal:** the list exactly as the 17 October exception allows.
- **Items:** .32, .34.
- **Files:** new `Model/RepairList.cs`, `Ui/JumbleLists.cs`, `Live/LabJumble.cs`, new `tests/D02RepairTests.cs`.
- **Add:**
  - `record RepairRow(int Index, string X, int D, string Y, string Z, long DeltaN, int Steps)`.
  - `RepairListModel.From(sample)` keeps the adapter's order verbatim. The model has no comparer and no sort or filter member.
  - No row is focused on open; arrow keys focus one.
  - Digits 1 to 9 are bound only in the context `repair.list.focused`; later rows go by focus and Enter.
  - ΔN is a tabular numeral in one weight.
  - Words: "Repair k of m", with m = 2 for a two-twist row and 3 otherwise; "Whole repair only"; the empty line with its second, coverage line; the cut-off line.
  - Focusing a row shows the step label and the moving set's count, per step. Stepping uses `view.preview.scrub` or the preview step key.
- **Check:**
  - `D02RepairOrderEqualsSample`, for every cap sample and all three variants;
  - `D02RepairModelHasNoSortMember`, by reflection over the type;
  - `D02RepairNoFocusOnOpen`;
  - `D02RepairDigitsOnlyInContext`;
  - `D02RepairEmptyNamesCapRevisionAndCoverage`;
  - `D02RepairCutOffIsNotEmpty`;
  - `D02RepairTwoTwistRowHasTwoSteps`.
- **Window:** the repair slot opens under the named-cap cell. Arrows move the focus, and the step label changes with the scrub key.
- **Checkpoint:** no.
- **Depends:** B3, B8. CR-14 for a row's step focus. Drawing a moving set's outline in a view waits for L1.

### B12. Staging a picked row, and the boundary
- **Goal:** copying a row makes a sealed, whole-only preview; the program never picks; research output is refused at preview.
- **Items:** .5, .33.
- **Files:** `Model/RepairList.cs`, `Model/Session.cs` (extend `PreviewModel`), new `Model/Boundary.cs`, `Ui/DraftLine.cs`, `Ui/SessionPanels.cs`, `Live/LabJumble.cs`, new `tests/D02BoundaryTests.cs`.
- **Add:**
  - `record SealedSpan(RepairRow Row)`. In the draft it can be removed whole and never edited. Typing the same twists yields an ordinary word.
  - `Stage(row)` is callable only from the pick handlers (digit, or Enter on the focused row). It writes a repair-pair record `{cap, revision, x, d, y, z}` to the preview and sets `commit.bar.whole_only`.
  - `enum Origin { Solver, Library, Research }` on a draft span. In a solving session, `PreviewModel.Admit` refuses `Origin.Research` with the sentence of ADR-D02.40.
  - `class BoundaryLedger`. It counts an event when a built-in path stages, focuses, orders or applies without a solver action. Every call site passes the user event that caused it.
- **Check:**
  - `D02StageOnlyFromPick`: staging without a user event throws and counts;
  - `D02SealedSpanRemovedWhole`;
  - `D02TypedTwistsAreOrdinaryWord`;
  - `D02WholeOnlyBar`;
  - `D02ResearchOriginRefusedInSolvingSession`;
  - `D02ResearchOriginRunsInResearchSession`;
  - `D02BoundaryCountZeroOverScriptedSession`.
- **Window:** press 2 in the focused list. The draft line shows one sealed span, and the bar reads "Whole repair only". Pasting a research word shows the refusal.
- **Checkpoint:** yes, C2.
- **Depends:** B11. CR-15 for `jumble.repair.stage`; the draft row drives the sample flow, and the engine write waits for the row and for K2.

### B13. Word flows and signatures
- **Goal:** the commit bar's word line, the part and twist counters, the folded tree, signatures.
- **Items:** .29, .31.
- **Files:** new `Model/JumbleWords.cs`, `Ui/DraftLine.cs`, `Ui/LibraryPanel.cs`, `Model/Library.cs` (`EffectSignature`), new `tests/D02WordTests.cs`.
- **Add:**
  - `record WordTree(string Name, int ExpandedLength, WordTree[] Parts)`, and `ExpandedLength` with its alphabet on the bar.
  - A preview counter "part j of p, twist k of n". Playback steps by part, and twist by twist inside an opened part.
  - `LongWordBound`: a labelled placeholder constant read from design data. Above it, numerals only.
  - The crossing sentence "On the lattice, ν = 0: labels pass to 600-cell-Full", shown only when reachability is shown.
  - Signatures: a lattice-to-lattice word shows its 35 slots and its ν row from `corrections.json` and the sample. A word leaving the lattice shows `numeral.undefined` with its length and alphabet.
  - The log tab's residual ticks at N = 0, else the undefined band with N.
- **Check:**
  - `D02WordLengthIsExpanded`;
  - `D02PlaybackStepsByPartThenTwist`;
  - `D02OffLatticeWordSignatureUndefined`;
  - `D02CrossingSentenceNeedsReachability`;
  - `D02Seed7001WordsPlayNineParts`, with the digests of section 6 shown per step.
- **Window:** play the seed 7001 restoration part by part. The digest and N change with each part until N = 0.
- **Checkpoint:** no.
- **Depends:** B4. X3 for the real bound.

### B14. The ν band and ν-corrections
- **Goal:** the band beside the rail at N = 0, and the Built-in library branch.
- **Items:** .28, .39.
- **Files:** new `Model/NuBand.cs` and `Ui/NuBand.cs`, `Ui/OrbitRail.cs`, `Model/Library.cs`, `Ui/LibraryPanel.cs`, new `tests/D02NuTests.cs`.
- **Add:**
  - `NuBandModel`:
    - 35 cells aligned with `OrbitRailModel`'s cells, one 0/1 mark each;
    - the 8 frame-group marks on their orbits' cells;
    - O17 and O28 as five-position dials;
    - nine fixed dots for the always-zero coordinates.
  - The mark shape is outside the ring family. Off the lattice the band shows the undefined numeral.
  - Library › Built-in › ν-corrections: 36 rows in id order with id, exact length and ν row. Shift+Enter previews one. Space marks a row into a set with a signed multiplicity. One chord calls `jumble.corrections.check` and shows "ν after this set" and "labels not predicted".
- **Check:**
  - `D02NuBandAlignsWith35Cells`;
  - `D02C5ValuesAreDialsNotBars`;
  - `D02NuBandUndefinedOffLattice`;
  - `D02CorrectionsAre36InIdOrder`;
  - `D02CorrectionSetCheckMatchesSample`: the sample holds one `correction_set_check` answer per `nu-c<id>` state.
- **Window:** with a `nu-c<id>` state the band stands beside the rail with marks and dials. In the library, mark two rows and check the set.
- **Checkpoint:** no.
- **Depends:** B1, B4.

### B15. Encodings: the tint channel, contact classes, height, off-lattice
- **Goal:** the mode switch, the legends and the sticky class rule, proved on the samples.
- **Items:** .20, .21, .23, .24.
- **Files:** new `Model/JumbleTint.cs` and `Ui/JumbleLegend.cs`, `Live/LabJumble.cs` (kept-view part), new `tests/D02TintTests.cs`.
- **Add:**
  - `enum TintMode { None, ContactClass, Height, OffLattice }`. It is one field, so modes cannot stack.
  - `ContactClasses.Assign(contact, previous)`. It is deterministic and sticky: a domain keeps the class stored for its domain ID, a new domain takes the lowest class free among its neighbours, and a seventh need raises an overflow state that reuses no class across a contact and states its count. The lattice domain is neutral. The assignment is saved in the kept-view part `frame.jumble`.
  - Legends: class swatches with domain and piece counts; the height ramp 2, 4, 6 and above, with the largest height's numeral; the off-lattice count, taken from the exact lattice flag.
  - The domain focus layer and "height at least k", from B10.
  - Because pieces are not drawn, each view border carries "tint: <mode> (legend only in this build: needs L1)".
- **Check:**
  - `D02TintModesExclusive`;
  - `D02ContactColouringIsProper` on every sample;
  - `D02FourClassesAtWj300AndDeepExports`;
  - `D02OneClassAtIa40`;
  - `D02ClassStickyAcrossCommitUndoRedoBranchReopen`, over the seed 7001 chain and W-J records 299 to 301;
  - `D02OverflowNeverReusesAcrossContact`, on a synthetic K7 graph;
  - `D02HeightRampOffByDefault`;
  - `D02OffLatticeFromFlagNotHeights`.
- **Window:** switch the tint mode from the palette. The legend changes; at the deep export it shows four classes and the height counts.
- **Checkpoint:** no.
- **Depends:** B1, B10. The mode row is an unlisted draft row (F7). Drawing waits for L1.

### B16. The view items that wait: unavailable states and captions
- **Goal:** every item that waits for L1, R5 or geometry shows a named unavailable state instead of nothing.
- **Items:** .8 is built here. It also builds the interim states of .9, .10, .17, .18, .25 and .26, which are mapped "waits" in section 4.
- **Files:** new `Model/JumbleViews.cs`, `Ui/JumbleHost.cs`, `Live/LabJumble.cs`, new `tests/D02UnavailableTests.cs`.
- **Add:**
  - `record ViewOffer(string RowId, string Name, string[] Missing)` for:
    - `view.preset.global.ring_sheet` and `view.preset.global.hopf_rings`;
    - the cut slice and its held split and radius shell;
    - `view.mode.raw`, with its clip and its antipode key;
    - the exposed faces;
    - the overlays.
  - Each palette row is present and disabled, with a reason that names the missing facts, such as "needs R5, L1 and a gate row".
  - Global's border reads "projection chain (600-cell-Full)".
  - Local's border and Tier 1 read "exposed faces unavailable", with the code `input.exposed_geometry_unavailable`.
  - The named-cap cell shows the certificate points as a count.
  - The two captions are stored now, for the day the views exist: "only row neighbours are face neighbours" and "No raw view certifies no holes; N = 0 does."
- **Check:**
  - `D02EveryWaitingRowHasANamedReason`;
  - `D02GlobalKeepsProjectionChain`;
  - `D02ExposedFacesUnavailableOnBorderAndTier1`;
  - `D02CertificatePointsShowAsCount`: at least 1,654 at the blocked-grip sample if S1 is among the samples; else the sample's own count;
  - `D02NoWaitingItemDrawsAPlaceholderPicture`.
- **Window:** the palette lists the chart presets, the slice and the raw view as unavailable with reasons. The borders carry the two state lines.
- **Checkpoint:** no.
- **Depends:** B4. CR-14 for these rows; L1, R5, the stereographic stage, the exposed-face geometry and the gate rows for the pictures.

### B17. Keys and terms
- **Goal:** the held seat keys in the key map, and the glossary check over every D02 word.
- **Items:** .42, .43.
- **Files:** `Model/KeyMap.cs`, `Ui/KeyMapPanel.cs`, `tools/looklab/data/design/keymap.json`, new `tests/D02KeyTests.cs`, new `tests/test_d02_words.py`.
- **Add:**
  - Four seat entries in `keymap.json` with `"held": true`, placeholder keys labelled as waiting for XM1, and fixed seat glyphs (tier 1 placeholders).
  - `KeyMapModel.HeldSeat(seat)` returns its 12 twist keys with their rotation orders as numerals. The panel shows them while the seat is held; the state `KeyCapState.Held` exists already.
  - The repair digits are listed only under the repair context. The aids sit on chords.
  - `test_d02_words.py` loads `jumble-words.json` and `command-labels.json`. Every term of art must be a row of `docs/progress/1.0/glossary/terms.json` or be in the test's `PENDING_TERMS` list, which holds the eleven requested additions and must be empty once LP4 lands them. Use the term check of `docs/progress/1.0/glossary/build.py` where it applies to a word list.
- **Check:**
  - `D02HeldSeatShowsTwelveTwistsWithOrders`;
  - `D02RepairDigitsOnlyInRepairContext`;
  - `D02NoDuplicateChordAmongD02Bindings`;
  - `python tests/test_d02_words.py`;
  - `python docs/progress/1.0/glossary/build.py --check` still passes; the glossary file is not edited.
- **Window:** open the key map (F12 sheet or the panel), hold a seat key, and its twelve twist keys light with numerals.
- **Checkpoint:** yes, C3.
- **Depends:** B2. CR-19 for routing a held seat with a twist key as one chord (the display is built now, the route is not). XM1 for the bindings. The glossary request for `PENDING_TERMS`.

### B18. The research pane: Records and Claims
- **Goal:** `pane.research` as a centre peer pane in Analyse, with its verdict words.
- **Items:** .3, .44, .45, .46, .49.
- **Files:** new `Model/Research.cs`, `Ui/ResearchPane.cs`, `Live/LabResearch.cs`, new `tools/looklab/data/samples/research/panel.json`, `tools/looklab/data/design/workspaces.json`, new `tests/D02ResearchTests.cs`, new `tests/test_d02_research_sample.py`.
- **Add:**
  - `interface IResearchSource`, with one method per existing research row: `research.archive.inventory`, `research.claims`, `research.check.report`, `evidence.word`, `research.runs`, `research.obligations`.
  - `frame.AddPane("pane.research", …)` with four tabs. Lists use `VirtualRows`, grouped by stage, then family.
  - A record opens in place with its finding paragraph and a folded `WordTree` of named parts.
  - A researcher's name is a separate field. Changing it changes no digest and no row id.
  - `enum Verdict` and `VerdictLine(verdict, scope)`, producing exactly the sentences of ADR-D02.46 on one wrapping line, in one weight, with no colour role.
  - "pinned: input <digest>" never turns stale on a commit. The branch is read-only.
  - Copy and save carry the note "Solving previews refuse it" and set `Origin.Research` (B12).
  - Every control calls `registry` by row id.
  - The sample is synthetic. Its field names follow `rust/c600-research/src/panel/answers.rs` and `wire.rs`, with one row per verdict word and `historical_check_incomplete` included. It is labelled `evidence: "synthetic"`.
- **Check:**
  - `D02VerdictWordsExact`;
  - `D02NotApplicableCountsAsNoCheck`;
  - `D02CutOffIsNeverAVerdict`;
  - `D02PinnedInputNeverStaleOnCommit`;
  - `D02RenameChangesNoDigest`;
  - `D02EveryResearchControlInvokesARow`;
  - `python tests/test_d02_research_sample.py`: every field name in the sample occurs in the two Rust files, read as text;
  - the words-per-screen limit for `analyse` is raised deliberately in `screen-words.json`, in the same commit.
- **Window:** the Analyse layout shows the Research pane. Open a record, then expand a Fail row to its first failing claim.
- **Checkpoint:** no.
- **Depends:** B12. LP5's rows are already in the table.

### B19. The research pane: Runs and Obligations
- **Goal:** runs with declared budgets, reproduction, obligations, and the two CR-17 controls.
- **Items:** .4, .50.
- **Files:** `Model/Research.cs`, `Ui/ResearchPane.cs`, `Live/LabResearch.cs`, `samples/research/panel.json`, `tests/D02ResearchTests.cs`.
- **Add:**
  - `record RunBudget(seconds, twists)`, declared before start and stored in the run row. A run at its budget ends `BudgetStopped`, with the cut-off words.
  - A seed field that accepts 7401 to 7499 and names the scored blocks. A depth field. The source line "reads the frozen research data and archive".
  - A reproduce control that calls `research.run.reproduce` and shows "reproduced: bytes equal" or "differs at byte k".
  - The Obligations tab, with `research.obligations` and `research.obligation.export`.
  - The control "open derived state in a free view", captioned "derived; reachability open". It is refused past the four-view envelope and is unavailable until its row exists.
  - The control "step the checker", which opens LP4's debugger pane on the per-claim trace. It is unavailable until its row exists.
  - History shows no research runs.
- **Check:**
  - `D02RunNeedsDeclaredBudget`;
  - `D02RunAtBudgetIsCutOffNotVerdict`;
  - `D02SeedOutsidePoolRefused`;
  - `D02HistoryShowsNoResearchRuns`;
  - `D02DerivedViewRefusedPastFourViews`;
  - `D02Cr17ControlsUnavailableWithReason`.
- **Window:** the Runs tab. Declare a budget, start the sample run, see it stop as cut off, then reproduce it.
- **Checkpoint:** yes, C4.
- **Depends:** B18. CR-17 for both controls.

### B20. The tuning panel: knobs, ramps, compare
- **Goal:** `panel.tuning` in the right dock, with live ramps and two slots.
- **Items:** .47 (first half; the item is mapped to B21, where it completes).
- **Files:** new `Model/Tuning.cs`, `Ui/TuningPanel.cs`, `Live/LabTuning.cs`, `Live/LabFrame.cs`, new `tests/D02TuningTests.cs`.
- **Add:**
  - `TuningModel` over the existing `DesignSystem`, `ThemeFamily`, `Knob` and `TokenOverrides`.
  - Parts: Family; Look; the five seed knobs (`tone`, `contrast`, `blackLevel`, `accentHue`, `gridIntensity`), each above a live ramp strip; Compare, with slots A and B, Swap and a held key that shows the other slot while held.
  - It opens from the palette as a tab in the current layout's right dock, beside History in Solve.
  - The existing Appearance panel stays (F11's default).
- **Check:**
  - `D02TuningOpensInRightDockOfCurrentLayout`;
  - `D02KnobChangeRebuildsRampsOnly`: no model or view state is touched;
  - `D02SwapIsAnInvolution`;
  - `D02HeldCompareRestoresOnRelease`;
  - the sweep passes.
- **Window:** move a knob and the ramps and the whole frame follow. Hold the compare key to see the other slot.
- **Checkpoint:** no.
- **Depends:** B2, B15. CR-16 for the slot and compare rows (draft).

### B21. The tuning panel: presets, Advanced, simulation, Save checks
- **Goal:** complete ADR-D02.47.
- **Items:** .47.
- **Files:** `Model/Tuning.cs`, `Ui/TuningPanel.cs`, `Model/Gallery.cs` (`ColourMetric`), new `tools/looklab/data/design/tuning-presets.json`, `tests/D02TuningTests.cs`.
- **Add:**
  - Presets: shipped ones are read-only, and an edit forks a copy.
  - Advanced, behind a switch, with search over all roles.
  - A held colour-vision and high-contrast simulation. It is a full-window filter in the lab and covers the chrome only in the shell (F10); the panel says which.
  - `Save()` runs three checks and fails in place, naming the failing pair:
    - the contrast rules of `system.json`;
    - a monotone lightness for `tint.jumble.height.{2, 4, 6, above}`;
    - pairwise distinct `tint.jumble.contact_class.*` under each G4 simulation, by `ColourMetric`.
  - The live-selector revert and the difference-file export, as draft rows.
- **Check:**
  - `D02SaveFailsOnNonMonotoneHeightRamp`;
  - `D02SaveFailsOnCollidingClassesUnderSimulation`;
  - `D02SaveFailsOnContrastRule`;
  - `D02ShippedPresetForksOnEdit`;
  - `D02AdvancedSearchFindsEveryD02Role`;
  - both themes pass `Save()` as shipped; if the placeholders do not, fix the placeholder values, not the check.
- **Window:** try to save a ramp with two equal steps, and the panel names the failing pair. Hold the simulation key.
- **Checkpoint:** no.
- **Depends:** B20. CR-16.

### B22. The reduced Track J driver: J1
- **Goal:** a scripted, model-free run that checks operability through command rows and keys only.
- **Items:** none new. It is the harness for .56.
- **Files:** new `tools/looklab/track_j.py`, `tools/looklab/lane_d02.py` (mode `d02-j1`), `Live/LabJumble.cs` (a script player that feeds key events), new `tools/looklab/data/samples/jumble/j1-session.json`.
- **Add:** the session file lists steps as `{keys or palette text, expect}`. The player sends real key events. After each step `Describe()` is compared with the expectation, which Python computes from the sample files independently of the C# loader. The four parts of J1 are in section 7.
- **Check:** `python tools/looklab/track_j.py --reduced --only j1 --out <new folder>` prints `LOOKLAB_D02_J1_PASS operations=<k> reached=<k> boundary=0 observation=<run>/<run> not_run=<list>`.
- **Window:** the owner can watch the script drive the lab in the live window with `--watch`. It is the same session at human speed.
- **Checkpoint:** no.
- **Depends:** B1 to B21.

### B23. J0 and the reduced run's report
- **Goal:** planted defects and the seed 7001 replay; one report.
- **Items:** .56.
- **Files:** `tools/looklab/track_j.py`, `Model/Jumble.cs` (a fault switch in `SampleJumbleSource`, read from `LOOKLAB_D02_FAULT`, compiled in lab builds only), new `tools/looklab/data/samples/jumble/defects.json`, `tools/looklab/README.md` (a D02 section), and a results page under `docs/progress/1.0/` beside the plan.
- **Add:** at least 20 defects, each `{id, what it changes, the rule it breaks}`. Section 7 lists the classes. For each one the driver reruns the J1 session and records "caught" when a comparison fails or the module refuses. The seed 7001 replay is B13's playback run by the driver.
- **Check:** `python tools/looklab/track_j.py --reduced --out <new folder>` prints `LOOKLAB_D02_J0_PASS planted=<n> caught=<m> replay=9/9` with m/n ≥ 0.8, plus the J1 line. The report lists every missed defect.
- **Window:** the final sitting. The owner watches the J1 session, then reads the two result lines.
- **Checkpoint:** yes, C5.
- **Depends:** B22. The ten sealed defects from C4 (section 3).

## 3. Checkpoints

Each checkpoint is one read of the named diff range with the questions below, about an hour. Findings follow the review schema. Only blockers and majors stop the next block.

**C1, after B5 (foundations).**
- Do the sample digests equal section 6, and does `--verify` rebuild them through the adapter only?
- Does `ToolSlot` satisfy J-7 and N-1 as written (late result, cancel of the latest, cancel of an older, coverage required)?
- Does the undefined numeral switch on the exact lattice flag only?
- Is the entry slot chrome only?
- Do the lab and the shell share one `JumbleSurface` with no copied file, and does the shell's fixed-point test see it?
- Did any step touch ADR-D01's files, the command table, a frozen chapter or an LP4 or LP5 crate (`git diff --stat` against the base)?

**C2, after B12 (the boundary block).**
- Does the Q3.b test assert all four points, and is the screen compared whole, not field by field?
- Does anything in the repair list order, weight, colour, pre-focus or pre-stage a row? Is there any code path to `Stage` without a user event?
- Are blocked caps undefined everywhere (map, readouts, covers, threshold counts)?
- Is the total N ever derived from cap values?
- Is research origin refused at preview admission, not later?

**C3, after B17.**
- Is the class rule proper, deterministic and sticky on the real samples? Does overflow never reuse across a contact?
- Does every waiting item show a named reason and no invented picture?
- Do the words equal the record byte for byte? Is `PENDING_TERMS` exactly the eleven requests?
- Are the held seat keys display only, with no route faked ahead of CR-19?

**C4, after B19.**
- Are the verdict words exact? Are NotApplicable, not-checked and cut-off never counted as a pass or a fail?
- Does every research control invoke a row?
- Is the synthetic sample labelled, and are its fields those of the Rust source?
- At this checkpoint Fable also writes ten planted defects for B23 and hands them to the integrator sealed. They are added to `defects.json` only after B22's driver is committed, so the catcher is not written to the list.

**C5, after B23.**
- Does the report count caught defects honestly? A crash does not count as a catch unless the defect is "a result without coverage" or similar.
- Are the observation rules that need L1 reported as not run?
- Does the item map of section 4 still hold against the built code?
- Do the tuning panel's Save checks fail in place on real collisions?

## 4. Item map

| Item | Mapping | Interim state or note |
|---|---|---|
| .1 | B2 | Reach is tested again in B22 |
| .2 | B4 | Free placement test for every D02 panel |
| .3 | B18 | |
| .4 | B19 | |
| .5 | B12 | |
| .6 | B4 | The lists inside the accordion come in B9 to B11 |
| .7 | B3 | |
| .8 | B16 | |
| .9 | waits for R5, L1, a gate row per preset | Palette rows present and disabled with the reason; Global keeps the projection chain (B16) |
| .10 | waits for R5, L1 | No chart, so no rule to apply; `jumble.half_turn` row exists (B2) |
| .11 | B4 | |
| .12 | B4 | Contact-class mode and never-hidden set show as armed in the legend only (F9) |
| .13 | B7 | After M-RING is redone in B7's precondition |
| .14 | B7 | |
| .15 | B8 | |
| .16 | B8 | |
| .17 | waits for L1, R5 | Row disabled with reason; the named-cap cell shows straddler and point counts (B16) |
| .18 | waits for R5, the stereographic stage, J10, RPI.36 | Row disabled with reason; caption stored (B16) |
| .19 | B8 | Picking certificate glyphs waits with .26 |
| .20 | B15 | |
| .21 | B15 | Legend and rule now; drawing on pieces waits for L1 |
| .22 | B6 | |
| .23 | B15 | Legend and focus-layer count now; drawing waits for L1 |
| .24 | B15 | Count now; outline drawing waits for L1 |
| .25 | waits for the exposed-face geometry, R5 for the tint | "exposed faces unavailable" on the border and Tier 1; slots exist (B2, B16) |
| .26 | waits for L1 and the certificate-glyph gate row | Certificate points as a count; never-hidden set and cut trace named as not drawn (B16) |
| .27 | B2 | Drawn in B7 and B8 |
| .28 | B14 | |
| .29 | B13 | |
| .30 | B10 | Layer, key and count now; dimming in a view waits for L1 |
| .31 | B13 | X3's bound is a labelled placeholder |
| .32 | B11 | |
| .33 | B12 | Engine write waits for CR-15 and K2 |
| .34 | B11 | Step label and count now; outline in a view waits for L1 |
| .35 | B9 | |
| .36 | B9 | |
| .37 | B2 | |
| .38 | B10 | |
| .39 | B14 | |
| .40 | B3 | |
| .41 | B2 | |
| .42 | B17 | Routing waits for CR-19; bindings wait for XM1 |
| .43 | B17 | |
| .44 | B18 | |
| .45 | B18 | Its two CR-17 controls are B19 |
| .46 | B18 | |
| .47 | B21 | First half in B20 |
| .48 | record only | Round 4; no row and no code |
| .49 | B18 | |
| .50 | B19 | |
| .51 | record only | |
| .52 | record only | The seed field of B19 enforces the pool |
| .53 | record only | |
| .54 | record only | J0 and J1 are run by B22 and B23 |
| .55 | record only | |
| .56 | B23 | The reduced run; the full run is after K2 |

## 5. Rows and data

**Rule for every missing row.** The row is a draft row in `tools/looklab/data/design/command-rows-d02.json` with its `request`. The lab and the shell register it, and the palette marks it "draft row". The test `D02DraftRowsAbsentFromTable` fails when the table gains the id, and the draft row is then deleted in a one-line commit. Draft ids are proposals to the top integrator, not names this build fixes.

| Request | Steps | Built before the row exists | Waits |
|---|---|---|---|
| CR-14 | B4, B8, B10, B11, B16 | Threshold set and clear, entry slot, straddler layer and repair step focus work on sample data through draft rows; slice, raw view and chart preset rows are present and disabled | The view rows' behaviour (L1, R5) |
| CR-15 | B12 | Sealed span, whole-only bar and repair-pair record in the sample preview | The engine preview and record write (K2) |
| CR-16 | B20, B21 | The whole panel on draft rows | Nothing in the shell; the product rows |
| CR-17 | B19 | Both controls shown, unavailable, with the reason "row not in the command table yet (CR-17)" | Opening the debugger trace and the derived view |
| CR-19 | B17 | Held-seat display with 12 twist keys and orders | Routing a held seat with a twist key as one chord; duplicate, prefix and layout checks |
| Glossary | B17 | `PENDING_TERMS` holds the eleven additions | The list empties when LP4's file has them |
| Unlisted (F7) | B8, B14, B15 | `jumble.cap.name`, `jumble.cap.unname`, the tint mode row, the two focus-layer rows and the correction-set mark as draft rows tagged `unlisted` | The integrator decides: add them to CR-14, or map them to existing rows |

| Data | Steps | Built before it exists |
|---|---|---|
| L1 (per-piece poses in Look Lab) | B10, B11, B15, B16 | Counts, legends, labels and named "needs L1" lines; no posed piece is drawn |
| R5 (chart, slice, stereographic stages; gate rows) | B16 | Disabled rows with reasons; projection chain kept |
| Exposed-face geometry | B16 | The unavailable state and its three slots |
| X1, X4, X5 (aid job timings) | B3, B9 | `SampleJobs` uses a tick clock; `slot.fill` stays off |
| X3 (long-word bound) | B13 | A labelled placeholder constant in design data |
| X6 (dim path) | B10 | The straddler layer is never a default |
| XM1 (bindings) | B17 | Placeholder keys labelled as such |
| Certificate-glyph gate row | B16 | The count in the slice's place |
| M-RING | B7 | Redone as a tracked test before the cap map rests on it |
| M-FOLD | none | Only the raw view rests on it, and that waits |

## 6. Sample data

All samples are written by `tools/looklab/fixtures/make_d02_samples.py` through `RustJumbleEngine` and checked by `tests/test_d02_samples.py`. Menu identity `fddc8532…958c` (I_a) is in every file.

| State id | Source | Must match | Used by |
|---|---|---|---|
| `on-zero` | `start(I_a)`, no twist | `record()['digest']`, pinned at B1; expected to equal the W-J fixture's `stages.start.digest` `e3b0c442…b855` (F12 decides) | B4, B6 |
| `wj-ia-300` | `research/jumbling/fixtures/wj-I_a.json`, journal records 0 to 299 | Digest pinned at B1 from two equal runs; contact classes exactly 4 | B4 to B12, B15 |
| `wj-ia-420` | Same journal to record 420 | Off-lattice pieces 114,930; digest against `stages.mid.digest` `940367a8b97ca199…` if F12 holds | Anchor only |
| `wj-ia-end` | Same journal, all 841 records | Off-lattice 121,184; 27 admissible and 573 blocked caps; `stages.end.digest` `6904a731ad8caa88…` if F12 holds | B7, B8, B10 |
| `ia-40` | Corrected by the integrator (F13): the I_a@40 stop of `stop40_reference()` in `research/jumbling/differential/references/obs_refs.py:125`, the W-J I_a fixture at 40 records then its 62 recorded restorer twists, replayed through the adapter | The reference's digest and off-lattice count if F12 holds; contact classes exactly 1 | B15 |
| `deep-1` | `research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json`, the 3,000-twist `history` | `1335f47e0c5647a948dd11f87ba4bad27c787d00992be1f783d970c88a542ace`; N 145,574; off 118,821; heights 116,665 / 1,971 / 185 | B6, B15 |
| `deep-2` | `deep-1`, then the record's 313 `moves`, then the 12 letters at `research/jumbling/differential/leads/results/summary.md:497` | `bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`; off 103,896; 102,960 at height 2; 936 at height 4 | B6, B15 |
| `s7001-0` to `s7001-9` | `…/11-scramble-census/runs/round1/ia-k10-s7001.json` `history` (10 twists), then `…/13-tutorial-runner/runs/depth10-seed7001/words/00000.json` to `00008.json` | Each word file's `input_digest` and `output_digest`, the first being `1438ebed…d6b1` to `ab254e2a…c7f9`; N 13,270 and off 26,519 at the start; solved at the end | B13, B15, B23 |
| `nu-c<id>` (two ids) | `on-zero` plus one ν-correction from `correction_lookup(id, twists=True)` | Digest pinned at B1; N = 0; ν equal to the correction's row | B14 |
| `corrections.json` | `corrections()` and `nu_constants()` | 36 rows in id order | B13, B14 |
| `cap-layout.json` | `model()` poles against `tools/looklab/fixtures/hopf-rings.json` | A bijection onto 20 rows of 30 | B7 |
| `research/panel.json` | Synthetic, shapes from `rust/c600-research/src/panel/` | Field names present in the Rust source | B18, B19 |

Notes:
- Each state has two or three named caps: at least one blocked cap and one admissible cap with a non-empty repair list.
- The W-J journal was written under the A1 to A4 contract revision. Replay its records through the same conversion `tests/test_jumbling_rust_candidate.py` uses for its W-J cases, not a new one.
- `deep-2`'s twelve letters are `[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[124,2],[274,13],[46,1],[46,3],[104,14]]`.

## 7. The reduced Track J run

**Form.** A scripted run with no model and no API. ADR-D02.51's researcher agent belongs to the full run. The driver is `tools/looklab/track_j.py`. It works only through key events and palette text sent to the lab, and reads only the lab's `Describe()` dump.

**Commands.**
- `python tools/looklab/track_j.py --reduced --out <new folder>`: both bars, headless, on the CPU lane.
- `python tools/looklab/track_j.py --reduced --only j1 --watch`: the same session at human speed in a window the owner starts.
- Output: `report.json`, plus the lines `LOOKLAB_D02_J1_PASS …` and `LOOKLAB_D02_J0_PASS …`.

**J1 on sample data.**
- *Keyboard reach.* For every row id starting `research.`, `jumble.` or `evidence.` and every D02 draft row, the driver reaches it from a fresh lab by keys alone (a chord, or the palette). It records that the row was invoked. `pointer_only` stays empty.
- *Observation.* One scripted check per rule of `jumbling-observation.md` that chrome can show: naming before any job, complete facts or explicit incomplete coverage, no method-selected object, kept checks in release order. Rules that need drawn pieces (rule 5's never-hidden set, the cut trace) are listed under `not_run` with "needs L1". They are never counted as passed.
- *The repair list's boundary.* For each cap sample the dumped rows equal the adapter's rows in order. No focus exists on open. Nothing is staged until a digit or Enter is sent. The empty and cut-off lines carry their coverage.
- *The boundary count.* `BoundaryLedger` reads zero at the end of the session.

**J0 on sample data.**
- *Planted defects.* At least 20, switched one at a time by `LOOKLAB_D02_FAULT`. Ten are written by Opus and ten by Fable, sealed at C4. Classes:
  - repair rows reordered, or one row pre-focused;
  - N_c shown at a blocked cap;
  - total N shown as the sum of the caps;
  - pending shown as undefined;
  - a late result overwriting a newer one;
  - a completed result without coverage shown as ready;
  - a folded row carrying a numeral;
  - a class reused across a contact;
  - a blocked cap counted as a threshold non-member;
  - a cut-off worded as "none";
  - NotApplicable counted as a pass;
  - research output admitted to a solving preview;
  - a digest changed by one character;
  - a ΔN sign flipped.
- A defect is caught when a driver comparison fails or the module refuses. The bar is at least 80%.
- *Seed 7001.* The driver plays the nine recorded words through B13's playback. At each step the digest the module shows equals the word file's `input_digest` before and `output_digest` after, and the last state is solved.

**What it proves.**
- The surfaces show the adapter's facts faithfully.
- The host rules hold: the slot states, N-1, J-8, whole-only repair.
- Every operation has a keyboard route.
- The boundary holds in the chrome.
- The chain of samples is intact.

**What it does not prove.**
- That the module executes a twist or computes a digest: the adapter did, at sample build. The true replay through the module is after K2.
- Anything about the engine wiring, timing, the four-view envelope or the renderer gate.
- Windows input: key events are injected.
- The researcher agent's ability, J2 to J4, or the target of ADR-D02.55.
- Any human evidence.
- That a model-free script is the independent "run" the bar means: that is a reading the owner should confirm (question 2 in section 8).

## 8. Risks and findings

**In the code this builds on.**
- **F1 (major).** The chrome library has none of the revision 4 and 6 host primitives ADR-D02 rests on: no tool-slot states, no undefined numeral, no whole-only bar, no practice mark, no pattern roles. A search of `tools/looklab/app`, `tools/looklab/core` and `tools/looklab/data/design` finds none of those names. `tools/looklab/app/Chrome/Model/Session.cs:11` has only four preview states. B2 and B3 build them as code and data; that is not an edit of ADR-D01's text.
- **F2.** `FocusLayers` at `tools/looklab/app/Chrome/Model/Gallery.cs:157` is a fixed list of pick layers, not a predicate layer with a count. B10 extends it.
- **F3.** The shell registers only the command table (`renderer/shell/project/ShellChrome.cs:124`), while the lab also adds draft rows and labels (`tools/looklab/app/Live/LabFrame.cs:119` to `:123`). `renderer/shell/build_shell.py:62` to `:64` stages no sample data. B5 fixes both.
- **F4 (minor).** `renderer/shell/README.md:57` says the host wrapper refuses a version other than 2, and its list ends at ABI 6 (`:144`). `renderer/bridge/host/src/lib.rs:106` says 7.
- **F14.** `tools/looklab/lane_a5.py:10` matches `parts=15` exactly, and `:6` sets the sweep limit to 400 s against about 230 s today. Every new kept-view part changes the first. D02's controls will lengthen the second. Give D02 its own sweep mode in `lane_d02.py` rather than raising the limit.
- **F15.** The research sample must be synthetic, because the only driver of the panel's real answers is `rust/c600-research/tests/panel.rs`, in an LP5 crate no step may edit.

**In the ADR-D02 text.**
- **F5.** ADR-D02.37 lists "`slot.fill` and `cue.check`, reserved rows" (`ADR-D02-jumble-research.md:299`). In the lines above it, `cue.*` names are cues, not rows. Section 16 repeats the pair (`:451`). B2 builds one off row, `slot.fill`, with the cue `cue.check` reserved.
- **F6 (major).** ADR-D02.13 rests the cap map on the ring-sheet arrangement with no data tag (`:142`), while `:73` to `:74` say M-RING is redone in the repository before an item rests on it. No tracked file I read maps cap ids to Look Lab's cells: `rust/c600-command/src/repair.rs:19` says only "as the jumbling engine numbers them". B7 carries the redo and the map as a precondition.
- **F7 (major).** CR-14 (`:429` to `:431`) asks for no row to name or un-name a cap (.16, .19), to switch the tint mode (.20), for the domain and "height at least k" focus layers (.21, .23), or to mark a ν-correction into a set (.39, `:308`). The table has no such rows. ADR-D02.1 requires every operation to be a row.
- **F8 (minor).** Section 15 goes from CR-17 to CR-19 (`:436` to `:438`). Say where CR-18 lives, so nobody looks for it here.
- **F9 (minor).** ADR-D02.12 arms the contact-class mode and the never-hidden set at entry (`:134` to `:135`) with no data tag, but both draw on pieces and so wait for L1.
- **F10.** ADR-D02.47's simulation "runs over the real frame" (`:369`). In the product shell the core's frame is a layer beneath the Godot window (`tests/test_renderer_shell.py:86`), so a chrome filter cannot cover it. It needs a core stage, hence a look-cost row.
- **F11 (minor).** `panel.tuning` overlaps the existing Appearance panel (id `appearance` in `LabFrame.cs`). The text does not say whether it replaces it. Default taken here: both stay.
- **F12 (major, decided by one assertion in B1).** Two digest functions may be in play. The W-J fixture's start digest and the seed 7001 runner's end digest (`research/jumbling/restoration/13-tutorial-runner/runs/depth10-seed7001/check.jsonl:10`) are both the hash of empty input. `tests/test_jumbling_rust_candidate.py:263` says J1 computes its own digest from a candidate record.
- **F13.** The "I_a@40 stop" (`:69`; `docs/progress/1.0/adr-d02-plan.md:18`) names no tracked file. The integrator names it. Until then, try W-J records 0 to 39 and accept it only if the class count is 1.
- **F16 (minor).** ADR-D02.6 says the Local lists expand "one at a time" (`:101` to `:102`), so the kept twist checks are closed while the repair list is open. B4 keeps a one-line header with counts on each closed list.

**Risks.**
- The words-per-screen limits (`tools/looklab/data/design/screen-words.json:6`, solve 155) will be passed by B6 to B11. Raise them deliberately per step.
- `seam_facts(all)` at the deep exports has no timing yet. B1 may need a budget and a stored cut-off.
- Both themes must pass B21's Save checks with placeholder values.

**Questions only the owner can answer.**
1. Does Jumble drawing enter Look Lab (L1) during this build or after it? The plan lists this as the owner's open answer. If during, items .17, .21, .23, .24, .26, .30 and .34 gain a drawing step after B16; if after, they stay as mapped.
2. Is a scripted, model-free run enough for the reduced J0 and J1 (recommended, no spend), or should a subscription subagent also act as the researcher for the reach and defect parts?

## 9. Estimate

| Step | Agent hours | Step | Agent hours |
|---|---|---|---|
| B1 | 6 | B13 | 4 |
| B2 | 4 | B14 | 5 |
| B3 | 5 | B15 | 5 |
| B4 | 5 | B16 | 3 |
| B5 | 4 | B17 | 4 |
| B6 | 3 | B18 | 6 |
| B7 | 5 | B19 | 4 |
| B8 | 5 | B20 | 5 |
| B9 | 5 | B21 | 5 |
| B10 | 4 | B22 | 5 |
| B11 | 5 | B23 | 5 |
| B12 | 4 | | |

- **Opus:** about 106 agent hours over 23 commits. Each step is one session of three to six hours.
- **Fable:** five checkpoints of about an hour each, plus half an hour at C4 for the ten sealed defects. About six hours.
- **The owner:**
  - five live-window sittings of 10 to 15 minutes, at C1 to C5, about one hour in all; C1 includes one launch of the product shell;
  - two answers (section 8), about ten minutes;
  - watching between sittings is optional: start `--run` and press F9.
- If the owner brings L1 into this build (question 1), add one drawing step of about six hours for each of the slice, the tint modes and the overlays, each with its own gate row.

## 10. Integrator's check and dispositions (26 November 2026)

**Check against the packet's acceptance.** Every item ADR-D02.1 to .56 is mapped once (section 4). Every step names
its items, files, check, window, checkpoint and dependencies. Steps are three to six agent hours. The row-dependent
steps are marked (section 5). No step edits ADR-D01, the command table, a frozen chapter or an LP4 or LP5 crate. Each
checkpoint has scoped questions. The 46 existing files and folders the steps cite exist on main `4202cbd9`, and the
adapter (`research/jumbling/differential/candidate/rust_jumble.py`) has all 16 methods the builder calls.

**Dispositions of the findings.**
- **F1 to F4, F14, F15, F12** (the code): adopt as written; each is carried by the step that names it.
- **F5:** adopt. ADR-D02.37 and section 16 now name one off row, `slot.fill`, with the cue `cue.check` reserved.
- **F6:** adopt. ADR-D02.13 now carries the data tag; B7's precondition does the redo.
- **F7:** adopt. CR-14 in ADR-D02 section 15 now also asks for rows to name and un-name a cap, switch the tint mode,
  turn on the domain and "height at least k" focus layers, and mark a ν-correction into a set. The top integrator
  decides the ids; the draft rows of B2 stay until then.
- **F8:** adopt. Section 15 now says CR-18 is ADR-D01's, decided by revision 6.
- **F9:** adopt. ADR-D02.12 now carries the L1 tag for anything drawn on pieces.
- **F10:** adopt. ADR-D02.47 now says the simulation covers the chrome only in the product shell until a core stage and
  its look-cost row exist.
- **F11:** adopt the default. ADR-D02.47 now says the Appearance panel stays.
- **F13:** resolved here: the stop is `stop40_reference()` / `stop40_state()` in
  `research/jumbling/differential/references/obs_refs.py:125`; section 6's `ia-40` row is corrected, and the ADR's
  facts section names it.
- **F16:** adopt. ADR-D02.6 now keeps a one-line header with its count on each closed list.
- **The two owner questions** of section 8 are put to the owner with the recommended option first.
