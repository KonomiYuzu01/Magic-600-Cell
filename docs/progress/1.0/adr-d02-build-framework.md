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
- `watch_d02.py --follow` keeps one window across steps (below). Without it, the owner closes and reruns the command after a step lands.
- Each step's "Owner sees" line says what to look at. Each checkpoint has a short sitting.

**The watching window never sits on top and never takes focus** (owner, 26 November 2026; added by the integrator). The owner works in other apps and switches to the window to look now and then.
- Every window the build starts for watching is a normal window: `watch_d02.py`, its `--follow` restarts, and the W runs of `d02_run.py` and `track_j.py`. It is never topmost: no `WS_EX_TOPMOST`, no always-on-top flag.
- It never takes the foreground: no activation, raise or `SetForegroundWindow`, neither at launch nor on a reload, rebuild or restart.
- `launch.py` starts the shell without activation: STARTUPINFO with `STARTF_USESHOWWINDOW` and `wShowWindow = SW_SHOWNOACTIVATE` (4). B4 added it with `--d02`.
- The shell must honour it. Godot's own showing of its main window may activate it, so B4 starts by reading how Godot 4.7's Windows display server shows the main window, and keeps it from activating. One lead: the main window's no-focus flag at start, cleared after the first frame without activating.
- `--follow` prefers reloading in place to restarting. A data-only change (samples, design data) is reloaded in the running shell. A new build is started without activation at the old window's rectangle, and the old window closes after it.
- Not covered: the gate rows (machine mark G) that need the foreground by design, as C9's `--topmost` probe does. They keep their own foreground rules and run in quiet windows arranged with the owner.

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
- **As built (integrator, 26 November 2026):** the start digests are equal, so the W-J stage digests check `wj-ia-420` and `wj-ia-end`. Three parts move to the step that consumes them:
  - `contact.json` moves to B15, since the domains' contact graph needs the seam-face geometry;
  - the hidden-piece masks move to B16, under `jumbling-observation.md` rule 5;
  - `cap-layout.json` stays with B8.
  The twelve release checks are the first twelve non-identity menu items at the named cap; B10 maps a seat to its items. The inside set and the repair rows' moving sets are read from the adapter's state, loaded into the simulator by `j1_from_record`'s reading; no state is replayed there.
- **Check:** `python tests/test_d02_samples.py` (no adapter) checks the format, section 6's digests and counts, and that the rim classes sum to 600. `make_d02_samples.py --verify` prints `D02_SAMPLES_PASS states=<n>`.
- **Owner sees:** nothing yet.
- **Machine:** H. About two minutes per deep state. The adapter build is a cargo build in the worktree's own target dir: never in a quiet window, and `cargo clean` for that target dir when the build line ends.
- **The owner's window rule:** B1 opens no window. From B4 on, every watching window follows section 1: never on top, never taking focus.
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
- **As built (integrator, 26 November 2026):**
  - The seven slot roles take revision 6's names, `slot.tool.{empty, pending, ready, stale, refused, budget_stopped, error}`, as ADR-D02.7 says; no `slot.tool.state.*` level.
  - The motion table's parser reads versions 1 and 2. A reserved row carries `"off": true`, a version 2 key, and takes no time.
  - 30 draft rows: CR-14 has 18, CR-15 one (`jumble.repair.stage`, a workspace row that never commits), CR-16 eight, CR-17 two and CR-19 one (`jumble.seat.hold`). All are label-only in `command-icons.json`, and A5's icon check also knows them.
  - `command-labels.json` shows six table rows by their display names. Making their table labels palette synonyms is B27's.
  - `jumble-words.json` holds 41 sentences with their items, 14 display names, the synonym, the six rows and one word the record forbids ("mirrored"). The check joins the record's line wraps before comparing bytes.
  - Theme values are labelled placeholders for round 3: contact classes on six hues, the height ramp on the neutral ramp in order.
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
- **As built (integrator, 26 November 2026):**
  - `Request` keeps the state from before the request. A pending request that a newer one supersedes keeps the state from before both, since its result is never taken.
  - `Finish` takes a result only for the latest open request, with the same digest and parameters, at the revision on screen. A `Cancelled` end acts like `Cancel` of the latest request.
  - `Invalidate(digest)` makes a shown result, a refusal or an open request stale. When the revision returns to a result's digest, that result shows again: this is the "stale until an undo returns to it" of section 1.
  - `SlotWords` reads the slot words from `jumble-words.json`; "pending" and "stale" are ADR-D01's state words. `ReleaseRecord.Superseded` says whether it recorded the release, and `IsSuperseded` reads it.
  - `Numeral` gives each kind its role and its Tier 1 word: pending draws with `state.pending`, incomplete with `slot.tool.budget_stopped`.
  - `ToolSlotBox` draws seven glyphs, none from the ring family. The console tests do not compile `Ui/`, so `python renderer/shell/build_shell.py` compiles it offline.
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
  - The owner's window rule (section 1): `launch.py` passes the no-activation STARTUPINFO; the shell keeps its main window from activating; `watch_d02.py --follow` reloads data in place and restarts a new build without activation at the old window's rectangle.
- **Check:** new cases in `tests/test_renderer_shell.py`:
  - D02 staging; the three launch options over the fake build; no D02 variable without `--d02`; no private path in `launch.json`;
  - the fake `popen` receives STARTUPINFO with `STARTF_USESHOWWINDOW` and `SW_SHOWNOACTIVATE`;
  - a source scan of `renderer/shell/` and the chrome library finds no topmost flag and no window activation outside the gate path.

  Then `python renderer/shell/d02_run.py --script smoke`, which also records the foreground window before the launch and five seconds after it. The two must be equal, and the shell's window must not carry `WS_EX_TOPMOST`.
- **As built (integrator, 26 November 2026):**
  - Godot 4.7.2's Windows display server shows a normal main window with `ShowWindow(SW_SHOW)`, then calls `SetForegroundWindow` and `SetFocus` on it. Its no-focus flag comes only from a project setting, with no command-line switch, and a project setting would also reach the measured C9 runs. So the shell does not stop Godot's activation; it undoes it. `launch.py --d02` starts the process with `SW_SHOWNOACTIVATE` and records the window that had the foreground in `M600_D02_FOREGROUND`. `D02/D02Window.cs` runs first in `Shell._Ready` and every frame for six seconds. At the first check it always hands the foreground back when the shell's own window has it, since only Godot's showing can have activated it then. At each later check it hands the foreground back the same way, and otherwise keeps its window just behind the foreground one, so a window shown without activation does not cover the owner's work. The later checks stop for good once a mouse button is down anywhere or the window moves or changes size (a drag, a maximise), so the owner's own clicks and switches stand; keys do not stop them, since keys typed elsewhere land in the shell while Godot holds the foreground. Windows makes a window topmost when it lands among topmost windows, so behind a topmost window (the taskbar, for one) the shell goes behind every topmost window instead (`HWND_NOTOPMOST`) and clears a topmost style at once. It stops the taskbar flash. Windows allows the hand-back, because the shell's process holds the foreground at that moment. `d02-window.json` and each dump record `gave_back` and `owner_acted`.
  - Four product builds on 26 November got there. The first checked every frame with no stop: its watched windows handed the foreground back four or five times while the owner was clicking them. The second stopped at any input, a mouse move included, and so left Godot's activation in place while the owner was busy in the Claude app. The third stopped at a mouse button or a moved window and handed back once, but a smoke run found its window topmost five seconds in, after it had been placed behind a topmost window. The fourth, `c9-product-d32a1f132dcaff14`, is the one above.
  - Extra launch options: `--d02-state ID` (the sample state the D02 surfaces will read, recorded by name), `--d02-data DIR` (a data folder the shell reloads from in place, for `watch_d02.py`), and `--position X,Y` (the watch command's restarts). `--d02` refuses `--cold-start`. `launch.json` records `d02`: the state, the fixture's and the script's SHA-256 and whether a reload folder was given, never a path.
  - `build_shell.py` stages through one `stage(project)` function, which also copies `tools/looklab/data/samples` to `project/samples`.
  - The palette marks a draft row "draft row", with the tooltip "Not in the command table yet: a row this build asks for". Invoking one answers "a draft row: this build does not carry it out yet". The frame shows a chip "D02 · <state>".
  - Script format `magic600-d02-script/1`: steps `{"chord": "F1"}`, `{"text": "tuning."}` and `{"wait_ms": 6000}`. The shell starts the script 1.5 seconds after it opens, injects each key through `Input.ParseInputEvent`, so it takes the shell's own route, and writes a dump eight frames after each step. A dump line in `d02-screen.jsonl` holds the step, the window's foreground and topmost state and how often the shell handed the foreground back, and the visible text, grouped as `frame`, `view:<name>`, `panel:<id>` and `overlay:<name>`.
  - `watch_d02.py --follow` checks every three seconds. A data change writes `d02-reload` in the run folder, and the shell reads the data again. A newer product build starts at the old window's place and size, which the shell writes to `d02-window.json`, and the old shell quits through a `d02-quit` file once the new one has written its own.
  - Smoke runs (`M600_D02_RUN_PASS script=smoke steps=5 mismatches=0` on the fourth build, `c9-product-d32a1f132dcaff14`): the window that had the foreground before the launch still had it five seconds later, and no window of the shell's process was topmost. `d02-run.json` now records the class of the foreground window before and after and of each shell window, never a title. Whether Godot's activation happens depends on the owner's machine at the moment: even a launch from Opus's background process, which descends from the Claude app, took the foreground while the owner was in that app. The runs saw both cases (`gave_back` 0 and 1).
- **Owner sees:** the frame opens with D02 on; the palette lists the draft rows.
- **Machine:** H, then W for the smoke run.
- **Checkpoint:** no.
- **Depends:** B2.

### B4a. ADR-D01's frame in the product on sample data
- **Goal:** the owner sees ADR-D01's whole frame in the product shell on sample data while D02 is built (owner, 26 November 2026: [the decision page](../../wiki/decisions/owner-decisions-2026-11-26-adr-d02-build.md), section 7). This pulls part of ADR-D03's scope into this pull request.
- **Items:** none new.
- **Files:**
  - `tools/looklab/app/Live/LabInspect.cs` split into new `SampleInspect.cs` (the shared sample inspector, Problems, Console and Tasks) and Look Lab's own View fields; `LabFrame.cs`, `LabScripts.cs`, `LiveSession.cs`, `Sweep.cs` follow the type;
  - new `tools/looklab/app/Live/SampleMount.cs`;
  - `renderer/shell/build_shell.py`, `launch.py`, `watch_d02.py`, `renderer/shell/project/ShellChrome.cs`;
  - `tests/test_renderer_shell.py`.
- **As built (integrator, 26 November 2026):**
  - The dock bug first (commit `af8a1bb1`): the shell gave the Library and History docks no content, so the Solve and Analyse layouts left them closed. The shell now puts its History on the right and the Library on the left, as the layouts name them, and the "Reference" layout is called "Learn".
  - `launch.py --sample` (needs `--frame`, refuses `--cold-start`, never in a measured run) sets `M600_SAMPLE=1` and records `sample` in `launch.json`. `watch_d02.py` always starts it.
  - With it on, `ShellChrome` builds a `SampleMount`, which mounts Look Lab's sample wiring on the shell's frame: the relationship strip on the seam and the orbit rail; the draft line with Goal and Net/Strict over the sample commit bar; the Library dock with its signatures; the inspector strip and panel; Problems, Console and Tasks with their badges and a sample task; the editor; the Scripts dock and the script panel slots; the key map drawn as a keyboard (help key held, or pinned); the pick-layer keys 6 to 0 and F10 beside each view; and the panel states on F12. A grey "sample" chip in the bar says so.
  - The real session keeps its own parts. `SampleMount.Handles` never takes `session.undo` or `session.redo`; the sample history is a dock panel of its own, "History (sample)"; the History dock, the position readout and the queue readout read the session's moves. The draft rows of `ShellJumble` are refused before sample mode is asked.
  - One copy: `build_shell.py` stages `LabDemo.cs`, `LabSolve.cs`, `LabScripts.cs`, `SampleInspect.cs` and `SampleMount.cs` from `tools/looklab/app/Live/` to `project/Live/`, and the sample programs, packages, key maps and the seed atlas they read to `project/sample-root/`. None of the five reads Look Lab's engine.
  - Not drawn: Global anchors (they need the core's projection), Local's own content (B18a to B18c) and the look and palette (B14).
- **Check:** `python tests/test_renderer_shell.py` (52 tests, with the new `SampleMode` and `ShellDocks` classes); `python tools/looklab/check.py` (LookLab.Tests 159 of 159) and `python tools/looklab/check.py --godot` (exit 0), since Look Lab's sources changed; `python renderer/shell/build_shell.py` and `build_product.py` (PASS, `c9-product-0d21b48f094c05dc`), and that build in the watching window in sample mode.
- **Owner sees:** the frame with the Library dock, the session's History, the strip and the rail, the draft line, the inspector strip, the sample chip and the task readout, in the watching window.
- **Machine:** H, W.
- **Checkpoint:** no.
- **Depends:** B4.

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
- **As built (integrator, 26 November 2026):**
  - `Turns::twist_letter(cap, item)` invokes `jumble.twist.menu` through the shell's route with the arguments a twist key gives (the router's `menu_item` letter for the pinned menu's two digests, the cap and the layer's revision), at any cap. `fixture::write_word` commits a word that way and closes with the control, as `write_for` does; its record adds `word` and `state_digests`. `Turns::state_digests` gives the head's digest and the SHA-256 of the pose ids (u32 little-endian per piece) and of the lattice flags (one byte per piece). The binary's `--word FILE` writes such a fixture, then reopens a copy once and prints `reopen_ms`.
  - Host ABI 8: `m6h_state_digest(handle)`, read through `m6h_answer`. `HostLibrary.cs` pins 8 and lists the export; `renderer/shell/README.md` no longer says version 2 (F11).
  - `build_d02_fixtures.py` rebuilds each state's word from the records its sample was made from. A W-J journal record is a menu letter: its grip and the base index of its label (`I_a[17]` is item 17). A replay of all 841 records as letters through the adapter reached `wj-ia-300`, `wj-ia-420` and `wj-ia-end` with the samples' digests before any fixture was written. Letters of the menu's identity item (item 0) are left out: they move no piece, and the command layer refuses them (`KN2:input.recipe_shape`); the stop-40 restorer has four, so `ia-40`'s word has 98 letters. The fixture's three digests must equal the sample's `state.json`, or it is removed. The two `nu-c` states wait for B26, whose correction words come from the adapter.
  - Written and matching on the integrator's machine (release build, write time and reopen time): `on-zero` (0 letters; 1.1 s, 0.4 s), `s7001-0` to `s7001-9` (10 to 19 letters; about 1.3 s, 0.3 s each), `ia-40` (98; 19.5 s, 5.3 s), `wj-ia-300` (300; 70.5 s, 17.5 s), `wj-ia-420` (420; 182 s, 37.8 s), `wj-ia-end` (841; 447 s, 139 s). The reopen time grows faster than the commit count (risk R2), so `deep-1` and `deep-2`, at about 3,300 commits, are not written at B5; they are written for the gate sittings, or once a faster reopen exists.
  - Written on 26 November 2026 after PR 454 made the command layer's jumbling replay linear (main d18b908a), at below-normal priority beside other work: `deep-1` (3,000 letters; 596 s, 233 s) and `deep-2` (3,321 letters, four identity letters left out; 701 s, 255 s), both matching their samples. On the old command layer `nu-c35` (5,629 letters) took 2,035 s to write and 1,244 s to reopen; the top integrator measured 156 s and 56.5 s for it on the linear one. A launch at a deep state still waits about four minutes to reopen.
  - `ShellJumble.Bind`: at each newly shown revision `Shell.cs` reads `m6h_state_digest` and binds the sample state whose two pose-array digests equal the head's, never by name; several states with equal poses (`on-zero` and `s7001-9`) bind the asked one. A chip says `bound · <state>` or `unbound · stale`. `watch_d02.py --state ID` and `d02_run.py` open the state's fixture when it is written.
  - Amended at checkpoint C1:
    - The key also takes the state digest (the answer's `digest` against the sample's `digests.adapter`). A pose id is a rank in a table compacted per state, so two states can share both pose-array digests.
    - Binding follows the revision each submitted frame draws, not the revision confirmed on screen: a window behind a window that fully covers it presents nothing.
    - An answer that does not read, or a failed call, unbinds and keeps the numerals undefined.
  - The bind run: the router turns cap 0 only (`grip.select` is not built), and cap 0 is blocked at `wj-ia-300`, so a twist key there is refused; on `wj-ia-300` the run shows the sample bound from the open. The whole cycle runs on `on-zero`: `M600_D02_RUN_PASS script=bind steps=6 mismatches=0` on `c9-product-6ef62b1b3f8d4577`, bound at the open, unbound after one twist, bound again after the undo, with the foreground kept and no topmost window. Script chords use Godot's key names (`J`, `Ctrl+Z`); a name Godot does not know is now written into the dump instead of being skipped.
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
- **As built (integrator, 26 November 2026):**
  - `Model/Jumble.cs`: `JumbleState`; `IJumbleSource`; `JumbleFeed`, the shell's source, which adopts the host's state-digest answer at each newly shown revision, with the lattice flag exact (`off_lattice == 0`); `SampleJumbleSource`, from a sample's totals, for the console tests; `JumbleNumerals`; `ChromeSettings`; `EntrySlot`; `JumbleChrome` (the mark and the mode cell's words); `LocalAccordion`. `EntrySlot` opens the cap map in the bottom dock and arms the contact classes and the never-hidden set. Its `Revert` gives back only the keys that still hold what it set, and it follows the adopted revision's mode.
  - The host's state-digest answer gains `off_lattice`, the count of pieces whose pose is off the lattice; with Jumble off it is null, as the two pose digests are. Host ABI stays 8, since only an answer field was added.
  - UI:
    - `ChromeFrame.SetModeCell` draws a glyph-and-words chip before the bar tools.
    - `JumbleSurface` is Local's content: the empty sentence, then the four closed headers with their counts, one open at a time.
    - `CapMapPanel` is the one new panel, in the existing bottom dock. It says "not drawn in this build yet" until B8, with a legend of the armed modes.
    - `JumbleHost` ties the entry slot to the frame's bottom dock in both directions, keeps the mode cell, switches the strip's and the rail's undefined numeral, and draws `mark.jumble` on each view border.
    - `RelationStrip` and `OrbitRail` gain `SetUndefined`, which shows `numeral.undefined` as a whole.
    - Two new icons: `mark.jumble` and `panel.capmap`.
  - Shell:
    - With `--d02` only, `ShellChrome` builds the host after `ShellJumble.Attach`. It puts the host's surface in Local, hands it the sample's strip and rail, and draws its marks.
    - `Describe` now clears Local's marks before drawing them; they used to pile up.
    - `Shell.cs` passes the shown revision to `Bind`.
    - The screen dump adds each view's mark tooltips, because with labels off a mark shows only its glyph.
  - Checks:
    - Look Lab's console lane: 165 of 165 pass, including the six named tests.
    - `tests/test_renderer_shell.py` (class `D02Host`): 61 tests OK.
    - `tests/test_render_ffi.py` passes.
    - `d02_run.py --script host`, in sample mode on each state's fixture on `c9-product-ba9fa71af300cc9f`, gives `M600_D02_RUN_PASS` with the foreground kept and no topmost window. On `ia-40`, off the lattice, it shows "Jumble · off the lattice" with the strip and the rail undefined. On `on-zero` it shows "Jumble · on the lattice" with true numerals.
    - `wj-ia-300`'s fixture reopens in about 17.5 s, after the script's first dump, so the off-lattice run uses `ia-40`. The watching window shows `wj-ia-300` bound.
    - One run failed its foreground check when the owner switched windows during it, as the check should. The rerun passed.
- **Owner sees:** the Jumble cell, a mark on each border, the strip and the rail undefined over the posed W-J state; true zeros on the zero-twist state.
- **Machine:** H, W.
- **Checkpoint:** yes, C1.
- **Depends:** B3, B5.

### B6a. The owner's view: rotation, the opening view and Local's shape
- **Goal:** the product window turns as Look Lab's does, opens on the owner's view of 22 November 2026, and draws Local as the owner set it (owner, 26 November 2026: [decision](../../wiki/decisions/owner-decisions-2026-11-26-adr-d02-build.md), section 8).
- **Items:** none new; ADR-D01.40's linked rotation once Local is drawn.
- **Parts and where they land:**
  - Rotation: shell only, here. The core already takes a 4D camera through `m6h_set_camera`; the shell had never sent one.
  - Gap 0.3 and sticker shrink 0.55, read from `tools/looklab/data/opening.json`: a per-view shape input brought forward from B14, on the drawing track.
  - The Hopf-ring opening at field of view 0.85: B23's chart path, pulled ahead of B22 on the drawing track. Behind `--d02` the window opens on it once it lands; as the product's default it waits for R5 (.8, .9).
  - Local from `tools/looklab/data/local-view.json` (orthographic, only the turning cells, pulled apart, centred at a fixed size, turning with the linked rotation): B18a to B18c on the drawing track, with picks wired in at B17.
- **Check:** `d02_run.py --script rotate`: two drags in the Global opening each turn the camera, a wait turns nothing, the foreground stays the owner's.
- **As built (integrator, 26 November 2026):**
  - `renderer/shell/project/D02/ShellCamera.cs` carries Look Lab's drag (`GlobalLayout.Drag` through `LookViewport.Drag`):
    - a left-button drag turns the 4D camera in the x–w and y–w planes, 0.004 rad per pixel, and the layout spin in the x–z and y–z planes;
    - a press turns nothing until it has moved three pixels, so a click stays a click;
    - the camera goes to the core as a column-major float4x4.
  - With `--d02` only:
    - `Shell._Input` routes a drag that starts in the Global opening, or anywhere in the window without the frame;
    - `_Process` sends a turned camera once per frame through `m6h_set_camera`.
  - The script player gains a `drag` step, and the screen dump gains `camera_turns`.
  - The focus hand-back cannot take a drag from the owner. It runs only in the window's first seconds, and it stops for good at the first mouse button the owner holds (`D02Window.Settle`).
  - `M600_D02_RUN_PASS script=rotate steps=6 mismatches=0` on `on-zero` (`c9-product-ae279d455a142133`), with the foreground kept and no topmost window.
  - `tests/test_renderer_shell.py` class `D02Rotation`.
- **As built, drawing track (integrator, 26 November 2026; merged as c75dd35a):**
  - Core ABI 6 adds `sa2_view_shape_set`: a view's cell shrink, sticker shrink, d4 and zoom, range-checked. A null pointer restores ABI 5's defaults (0.76, 0.82, 1.18, 1.15). A view whose shape was never set draws byte-identical to ABI 5, and the shaders are unchanged. Host 9 exports `m6h_set_view_shape`.
  - The shell's `D02/ShellShape.cs` reads the owner's values from `opening.json`, under `M600_D02_DATA` when it holds one, else from the build's staged `look/opening.json`: cell shrink 1 − 0.3 = 0.7 and sticker shrink 0.55. The values stay in that file, so they stay tunable. `Shell.cs` applies the shape right after `m6h_set_viewport`, and only with `--d02`.
  - d4 and zoom keep the core's values: the file's field of view (0.85) belongs to the Hopf-ring layout, which the core does not draw yet. The Hopf-ring opening and Local's shape still wait for B23 and B18c.
  - Checks: `check_core.py --shape` passed on WARP on the drawing track: the default shape is byte-identical to no call, a smaller sticker shrink covers fewer pixels, and a bad value is refused. After the merge, `d02_run.py --script rotate` passed on on-zero (`c9-product-6c1a32eaad1b01e4`).
  - B8's cap map had made the bottom dock so tall that a 1280×720 window left Global almost no opening, and rotate failed on the B8 to B10 builds. The cap map's words now sit beside the map, and its cells are two pixels shorter, so rotate passes again.
- **Owner sees:** drag in the watching window and the puzzle turns.
- **Machine:** W.
- **Checkpoint:** no.
- **Depends:** B4a.

### B7. Totals and the named-cap readouts
- **Goal:** the border numerals, with pending, incomplete and undefined apart.
- **Items:** .22.
- **Files:** new `Model/JumbleReadouts.cs`, `Ui/JumbleReadouts.cs`, `tests/D02ReadoutTests.cs`.
- **Add:**
  - `GlobalTotals`: N with its two parts, domains, off-lattice pieces, n2, n4, n6, and "above 6" with its largest height.
  - `CapReadout`: In, Out, A, N_c, floor, u.
  - Fixed tabular slots, one neutral change mark, the unit word "seams", no colour by value.
- **Check:** `D02TotalNPendingUntilSeamFactsAll`; `D02TotalNIncompleteAfterCutOff`; `D02TotalNIsNotSumOfCaps`; `D02BlockedCapReadoutUndefined`; `D02Above6MaxUndefinedAtZero`; `D02TotalsMatchSample`.
- **As built (integrator, 26 November 2026):**
  - `Model/JumbleReadouts.cs`:
    - `SeamCoverage`, which makes N and its two parts pending before `seam_facts(all)` covers the state and incomplete after a cut-off;
    - `GlobalTotals.Read(totals, coverage)`, which reads N from the state's totals, never from the caps;
    - `CapReadout.Read(capRecord)`: a blocked cap has In, Out, A, N_c and the floor undefined, and its u is the straddler count; an admissible cap has u = 0; a cap with no certified rim row is pending throughout;
    - `ReadoutChanges`, one neutral mark for a value that changed; a slot that held no value before is not marked.
  - `Ui/JumbleReadouts.cs`: `ReadoutRow`. Each slot has a label, the change mark, and a right-aligned numeral in a slot six digits wide. A slot with no value says its state's word in that state's role. Nothing is coloured by value, and nothing is a gauge.
  - `JumbleHost` puts the totals on Global's border and the named cap's readout on Local's while Jumble is on. A rebuilt frame gets fresh rows.
  - In the shell, the totals follow the binding and are pending while nothing is bound. The words "seams" and "above 6" come from `jumble-words.json`. The cap readout waits for B9, where a cap is named.
  - The sample builder accepts only complete seam facts. So the shell reads the bound state's totals as covered, and the pending and incomplete readings are tested on the model.
  - Checks: the console lane passed 171 of 171, with the six named tests. `d02_run.py --script host` passed on `ia-40` and `on-zero` (`c9-product-fabd043e0bc26316`). On `ia-40` Global's border reads N 1192 seams, 1057 on grip, 135 off grip, 6 domains, 860 off the lattice, n2 860, n4 0, n6 0, above 6 0, and max undefined.
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
- **As built (integrator, 26 November 2026):**
  - `make_d02_samples.py` writes `cap-layout.json` (`--layout` writes only it). Cap c is cell c of `assets/model.npz`; the builder reads the 20 rings of `hopf-rings.json`, checks that its model digest is the shipped model's, that the rows are a bijection onto 20 rows of 30, and that each row's neighbours, the last column with the first, are face neighbours by the cells' normals. `test_d02_samples.py` repeats both checks against `model.npz`. M-RING is redone as that tracked test.
  - `Model/CapMap.cs`:
    - `CapLayout.Read`, which refuses anything but a bijection onto 20 rows of 30; `At` wraps the columns.
    - `CapMapModel.Read(rim)` with the builder's rule: blocked, closed (N_c = 0), at floor, above floor. A row counts only in a complete record and only when it is certified (blocked or admissible); an uncertain or missing row, or an admissible row without N_c, leaves its cap pending.
    - The numeral: N_c at an admissible cap with N_c > 0. A closed cap is a plain cell (its N_c = 0 is its class and its tooltip), and a blocked or pending cap shows none.
  - `Ui/CapMapControl.cs`: one `_Draw` of the 600 cells. Blocked is the dark `pattern.cap.blocked` ground with a level bar; closed a plain cell; at floor a corner bracket and its N_c; above floor its N_c numeral only; pending ADR-D01's pending ring. A dashed wrap mark runs down both sides. Each cell keeps room for a four-digit numeral (the samples reach N_c 1,368). Hovering a cell shows its cap, ring, place, class and N_c; naming a cap waits for B9.
  - `CapMapPanel`: the head "Cap map · 20 rings of 30 caps", the map, a legend of the five classes with their counts ("at floor (N_c = F_c > 0)") and the wrap mark, then the armed-modes legend.
  - In the shell, the map follows the binding with the totals: the bound state's `rim.json` and the staged `cap-layout.json`. Every cap is pending while nothing is bound, and also for a sample written without its rim rows: only `wj-ia-300`, `wj-ia-end`, `ia-40`, `deep-1` and `deep-2` carry them, so `on-zero` shows 600 pending caps.
  - Not built here: the free pane. The frame has no free panes yet, so the cap map stays in the bottom dock; the item goes on the refinement round's list.
  - Checks: the console lane passed 176 of 176, with the five named tests; `test_d02_samples.py` 10 of 10 and `test_renderer_shell.py` 63 of 63. `d02_run.py --script host` passed on `ia-40` and `on-zero` (`c9-product-a284d6f724cc78d1`); on `ia-40` the legend reads blocked 25, closed 560, at floor 8, above floor 7 and pending 0, as its totals count them. The first `on-zero` run stopped because another window took the foreground in its first five seconds; the rerun passed.
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
- **As built (integrator, 26 November 2026):**
  - `make_d02_samples.py` writes `meeting-caps.json`: J1's `meeting_caps` for every cap, from the model's poles (cap d meets cap c when n_c·n_d is at least the 81st largest n_0·n_x). The float products fall on separate levels, so the builder takes the threshold between them, and it checks that every cap meets 80 and that the relation is symmetric. `test_d02_samples.py` checks it again, and checks that every named cap's covers record has the same domain.
  - `Model/CapThreshold.cs`:
    - `CapPredicate` reads the three formulas, `N_c ≥ k`, `N_c ≤ k` and `N_c = F_c`, also written `>=` and `<=`. Any other formula is refused with the reason.
    - `CapThreshold` is a set of predicates that a member meets together. There is no default. `Counts()` gives members, non-members, blocked and pending apart; a blocked cap is never tested. Members come in cap-id order.
    - `CapNaming` holds the named cap, plus a hover that names nothing. `MeetingCaps` refuses anything but 600 symmetric rows of 80.
    - `CapJobs` keeps every Jumble slot Empty until a cap is named.
    - `CapFacts` gives Tier 1 (the kind, the id, and N_c, the floor and u) and Tier 2 (the facts in one fixed order).
  - `CapMapModel` keeps the certified row's In, Out, A, N_c and F_c, and gives the named cap's readout. At a blocked cap, u stays pending unless the cap's own record gives its straddlers.
  - The map draws, over the patterns:
    - the Query set outline at each threshold member;
    - the named cap with ADR-D01's selection brackets;
    - each of its 80 meeting caps with a small square in the cell's corner, which is neither an outline nor a dim.
  - A click on a cell names a cap, and so does typing its number and pressing Enter while the map has focus. Hovering only inspects.
  - The panel has the threshold field with its four counts, and the named cap's Tier 1 line with its meeting caps. Local's Named cap section counts the named cap and shows the Tier 2 facts when it is open. The empty sentence goes once a cap is named.
  - While the dock is closed or holds another panel, the threshold field and its counts sit on Global's border.
  - The four CR-14 draft rows, `jumble.cap.name`, `jumble.cap.unname`, `jumble.capmap.threshold.set` and `.clear`, now work through the palette, which still marks them as draft rows. The host carries them out without invoking the command registry. The other draft rows still say they wait for their steps.
  - Picking a sticker and the frame's docked inspector wait for B17. Until then the tiers show in the panel and in Local.
  - Checks:
    - The console lane passed 182 of 182, with the six named tests. `test_d02_samples.py` passed 11 of 11 and `test_renderer_shell.py` 65 of 65.
    - `d02_run.py --script name-cap` passed on `ia-40` by keys only (`c9-product-b25915b566040b47`). `> jumble.cap.name 30` reads cap 30 · N_c 165 · floor 41 · u 0 · 80 meeting caps. `> jumble.capmap.threshold.set N_c >= 50` reads members 9 · non-members 566 · blocked 25 · pending 0, as the rim rows count them.
    - `--script host` passed again on `ia-40`.
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
- **As built (integrator, 26 November 2026):**
  - `Model/TwistChecks.cs`:
    - `TwistCheck` reads a release-check row: the twisted cap's N_c after the twist, the floor, ΔN, Δn_h and the named cap's Δu.
    - `KeptChecks` has `Release`, `Complete` and `Cancel`. A release supersedes the pending check and folds it in the same call, from the release record alone. A run of consecutive superseded releases is one `FoldedRow`, and a completed row or a cancelled release ends the run. Only the latest release's result is taken. Cancelling the latest withdraws it (J-7), and cancelling any other release is a no-op (N-1).
    - `CheckRow` carries the try number at the cap and revision, superseded releases counted. `FoldedRow` holds only its releases and twists.
    - `SampleJobs` is a tick clock that completes each request three ticks later.
  - `Ui/JumbleLists.cs`: `TwistCheckList` shows the columns `try | twist | N_c | floor | ΔN | Δn_h | Δu`, a completed row with its session release on hover, and a folded row "not checked:" with the twists' glyphs (`cap·item`). Local's Twist check section shows it and counts its rows.
  - The host keeps one list per named cap and revision, and the shell ticks it every 150 ms.
  - `jumble.seat.hold 1` (CR-19's row) stands in for holding seat 1 and pressing its twelve twist keys, until CR-19 routes the chords. It releases the cap record's twelve twists at the test's rhythm and opens the Twist check section. Other seats and caps without a record say why they are unavailable.
  - Not built here: the word check's collapsed row ("lifted"), which needs a word's sample record; it goes with the repair steps in B12 and B13.
  - Checks:
    - `D02Q3bTwelveTwists` runs on wj-ia-300 at cap 150 and asserts the four points of ADR-D02.36: completed rows in release order, each folded row written in the frame of its superseding release, folded rows with glyphs only, and the screen identical over every cancel of an older release. It also checks J-7, the try numbers and the cells.
    - The console lane passed 183 of 183, and `test_renderer_shell.py` 66 of 66.
    - `d02_run.py --script q3b` passed on wj-ia-300 (`c9-product-60f9ef04a1776cf7`). It printed `screen_changes_on_cancel=0 cancels=99`, and the kept list equals the rhythm's own result: four folded runs and the completed checks 3, 5, 9 and 12. The first run failed only its foreground check, because another window took the front in the first five seconds; the rerun passed.
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
- **As built (integrator, 26 November 2026):**
  - `Model/CapCovers.cs`:
    - `CapCovers.FromRecord` reads a completed `covers` answer from a cap record: its 80 meeting caps in cap-id order, each with "d covers c" and "c covers d" as yes, no or undefined with its reason. A blocked named cap's record has no completed answer (J1's job is an error there), so the slot shows none.
    - The slot's four lines: the domain line, then "covered by", "covers" and "undefined" in cap-id order. Each list says how many of the 80 it answered, so an empty list never reads "no" where answers are undefined. The three heads are the word record's (`covers.covered_by`, `covers.covers`, `covers.undefined`). `Table()` is the expanded form: 80 rows, `cap | covers c | c covers`.
    - Reading: ADR-D02.38's "alignment-domain line" is built as the answer's domain statement that JI.12 and ENG-0J's M4 require: "The 80 caps that meet cap c · at least 10 aligned face pieces a side". This goes to the refinement round as a named reading.
    - `CoversInspect` is the hover's own state: hovering an id shows that cap's Tier 1 on the slot's inspect line, and it touches neither the named cap, the cap map's hover nor its marks (`CapMarks`).
    - `PoseStyle` reads a state's pose rows, and `JumbleFocus` builds the three predicates with their counts: the named cap's straddlers (the record's straddler mask at a blocked cap; 0 at an unblocked cap, where the layer dims nothing; pending without a record), one alignment domain, and height at least k (the pieces' left height, as the totals' n_h count it).
  - `Model/Gallery.cs`: `FocusLayers.AddPredicate(id, label, count, applies)`, `Focus(id)` (the focused predicate again un-focuses; none by default), `Dimming` and `ShownPiece(member)`. The predicates use the existing cycling key: while one is focused, the cycle's dimmed or hidden mode applies to its non-members. `LayerKeys` shows each predicate as a key with its count, after the cycling key.
  - `Ui/JumbleLists.cs`: `CapCoversList` under the named-cap cell, with hoverable ids, the inspect line and the table behind a fold.
  - The host carries the draft rows `jumble.focus.straddlers`, `jumble.focus.domain <id>` and `jumble.focus.height <k>` (CR-14). A row adds its predicate to both views' layers and focuses it; the same row again un-focuses it. The straddlers recount at every named cap and stay focused.
  - Departure: the product shell outside sample mode had no focus layers, so the host keeps one set per view there (`OwnLayers`), with the cycling key on F10 as in the sample and the predicate keys only (no pick layers, which the product does not draw). The drawing of all three layers comes in B15.
  - Checks:
    - Console lane: `D02CoversUndefinedIsListed`, `D02CoversTableHas80RowsBothDirections`, `D02HoverInCoversMarksNoCapMapCell` and `D02FocusLayerCountsMatchSample` pass; the lane passed 187 of 187. The counts are cross-checked against the totals: height at least 2 equals the off-lattice count, and the lattice domain holds exactly n_0's pieces.
    - `test_renderer_shell.py` passed 73 of 73, with `D02Covers`.
    - `d02_run.py --script covers` passed on wj-ia-300 (`c9-product-568c5effe6cd5b26`): cap 150 is covered by 36 of 80 meeting caps, covers 0 of the 3 answered, and 77 answers are undefined (blocked). Hovering cap 8 inspected it with the cap map's marks unchanged. The focus keys read "Height ≥ 4 · 3,128", "Domain 22 (lattice) · 56,777" and "Straddlers · 0" in both views.
    - On the same build, `rotate` (on-zero), `name-cap` (ia-40) and `q3b` (wj-ia-300) passed again.
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
- **As built (integrator, 26 November 2026):**
  - `Model/RepairList.cs`:
    - `RepairRow(Number, X, D, Y, Z, ΔN)`; a row whose z is the identity is a two-twist repair with two steps, and its z is data.
    - `RepairListModel.From(record, revision, variant)` reads one of the sample's three results (completed, cut_off, cancelled) and keeps the record's order. It has no comparer and no sort, order, filter or rank member. ΔN is one signed numeral in its own column.
    - No row is focused on open. `Down` and `Up` focus rows inside the list. `Digit(n, contexts)` picks rows 1 to 9 only in the context `repair.list.focused`, and `Enter(contexts)` picks the focused row in the same context.
    - The words are the word record's: the columns `# | x d | y z | ΔN`, "Whole repair only", the empty line "Cap c, rev r: no repair lowers N (k of k pairs tried)" and the cut-off line "Not found within budget (j of k tried)". The total is 59 menu items times the misaligned meeting caps in the result's coverage.
    - The coverage line names the pin (the order digest), the menu identity, the tie rule and the contract revision. It always shows under an empty list, and behind a fold otherwise. A completed result without coverage, or one that tried less than everything, is an error.
    - `RepairSteps` gives each step's letters and moving-set count from the record's `repair_steps` (rows 1 to 9 in the samples; pending beyond). `RepairScrub` holds the focused row's step, "Repair k of m", moved by the preview's step keys.
  - `Ui/JumbleLists.cs`: `RepairListControl` takes its own keys while it has focus. Up and Down focus rows; Left and Right are `view.preview.scrub`'s keys and walk the focused row's steps; digits and Enter pick; Escape un-focuses. The step line under the rows reads "Repair k of m · moving n".
  - Departure: the shell routes every key the chrome does not claim to the session, so digits typed into the cap map (B9) or the repair list would also have twisted. `ChromeFrame.Claims` now claims keys while the focus owner implements `IOwnKeys`, and both controls do.
  - The host rebuilds the list for the named cap at each revision from the cap record. A pick reaches `JumbleHost.Pick` only from the list's digit or Enter, and B13 stages it.
  - Checks:
    - Console lane: `D02RepairOrderEqualsSample`, `D02RepairModelHasNoSortMember` (reflection), `D02RepairNoFocusOnOpen`, `D02RepairDigitsOnlyInContext`, `D02RepairEmptyNamesCapRevisionAndCoverage`, `D02RepairCutOffIsNotEmpty` and `D02RepairTwoTwistRowHasTwoSteps` pass; the lane passed 194 of 194.
    - `test_renderer_shell.py` passed 75 of 75, with `D02Repair`.
    - `d02_run.py --script repair` passed on wj-ia-300 (`c9-product-b724d282887e16dc`). The list counted 129 rows with none focused on open. Two arrows focused row 2, the forward step key showed "Repair 2 of 3 · moving 2,862", and typing 2 in the list picked row 2.
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
- **As built (integrator, 26 November 2026):**
  - `Model/Boundary.cs`:
    - `UserEvent`: the key or click that asked for an act. Only input handlers make one.
    - `BoundaryLedger` counts every staging, focusing, ordering or applying that names no user event, and refuses an act it does not know.
    - `enum Origin { Solver, Library, Research }`, and `PreviewModel.Admit(origin, solving)` in `Model/Session.cs`, which refuses a research run's output in a solving session with the word record's sentence.
    - `StagedWord` holds letters with `SealedSpan`s. A typed twist is an ordinary letter, even when it spells a repair row. Removing a letter inside a span removes the whole span, and editing inside one is refused.
    - `RepairStage.Stage(list, row, steps, asked)` stages a picked row as one sealed span and writes a `RepairPairRecord` to the sample preview: the cap, the revision, the row, x, d, y, z, ΔN, the pin, the menu, the tie rule and the asking key. Without a user event nothing is staged and the ledger counts the act. While a span is staged, the bar reads "Whole repair only" and any prefix through it is refused.
  - `RepairListModel.Down` and `Up` take the key that asked and record each focus on the ledger.
  - The host stages only in `Pick(row, asked)`, which only the list's digit or Enter handler reaches. `test_renderer_shell.py` pins that the UI has one `Stage(` call, and that it sits in `Pick`.
  - Departure: the framework names `Ui/DraftLine.cs`, but the product shell has no draft line until K2 (the sample's draft line exists only in sample mode). The staged word and its bar show in Local, under the sections, as `RepairStageBar`, with a control that removes the last span whole. CR-15's engine preview and record write still wait for the row and K2.
  - Fixed on the way: the new controls subscribed to their models in their constructors and unsubscribed on `TreeExiting`. A frame rebuild takes Local out of the tree once, so the stage bar stopped hearing the stage. They now subscribe on `TreeEntered`.
  - Checks:
    - Console lane: `D02StageOnlyFromPick`, `D02SealedSpanRemovedWhole`, `D02TypedTwistsAreOrdinaryWord`, `D02ResearchOriginRefusedInSolvingSession` and `D02BoundaryCountZeroOverScriptedSession` pass; the lane passed 199 of 199.
    - `test_renderer_shell.py` passed 76 of 76.
    - `d02_run.py --script repair` passed on wj-ia-300 (`c9-product-d665047970de3a1d`). Typing 2 in the focused list staged `[150·1 215·21 150·1]` as one sealed span, the bar read "Whole repair only", one repair-pair record was written, and the ledger counted 3 asked acts and 0 unasked.
    - On the same build, `covers`, `q3b` and `host` (wj-ia-300), `name-cap` (ia-40) and `rotate` (on-zero) passed again.
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
- **As built, drawing track (integrator, 26 November 2026; merged as c75dd35a):**
  - Core ABI 7. `sa2_style_set` takes one byte per piece per channel: the fill is a palette index in the low four bits, with bit 7 for the dim set, and the outline is a palette index. The style is bound to the adopted revision, and an adoption drops it.
  - `sa2_view_style_set` fills style slots 1 to 4 (316 bytes): tint and dim modes, the dim amount, the outline width, a 16-entry palette and a 3×3 output colour matrix. A one-view frame uses slot 1.
  - A frame draws styled only when its slot changes something; otherwise it keeps the unchanged product pipeline. The styled pass (`shaders/styled.hlsl`) is embedded as DXIL, so the identity's eight shader files and the acceptance harness's copies are unchanged.
  - Host 10, not the framework's 9, since 9 went to the shape: `m6h_set_piece_style` (per piece, or per pose expanded with the head's pose index) and `m6h_set_view_style`. `CoreLibrary` and `HostLibrary` bind them; nothing calls them until B15.
  - Checks: `check_core.py --style --shape` passed with 39 CPU checks on the drawing track, with the core, shell and acceptance tests.
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
  - Each state's `contact.json` (domains, contact edges with off-grip contacts), added to `make_d02_samples.py` here from B1.
  - `ContactClasses.Assign(contact, previous)`: proper, deterministic, sticky by domain id; a seventh need is the overflow state with its count; the lattice domain is neutral. Saved with the view state.
  - Drawing: contact class on the outline channel, so the fill keeps the label; the height ramp on the fill tint; off-lattice as a dark outline taken from the bundle's `pose_lattice`.
  - Focus dim through the dim mode. The straddler layer is never a default and is active only at a blocked cap.
  - Legends with counts.
- **Check:** `D02TintModesExclusive`; `D02ContactColouringIsProper`; `D02FourClassesAtWj300AndDeepExports`; `D02OneClassAtIa40`; `D02ClassStickyAcrossCommitUndoRedoBranchReopen`; `D02OverflowNeverReusesAcrossContact`; `D02OffLatticeFromFlagNotHeights`; `D02StraddlerLayerOnlyAtBlockedCap`. Then `d02_run.py --script tint`: the style bytes sent to the host equal the sample's classes, piece by piece.
- **As built (drawing track, 26 November 2026):**
  - Samples: each full state has `poses.json` (each piece's pose row, u32 little-endian, its sha256 the state's `pose_ids_sha256`) and `contact.json` (the domains, the lattice domain, the edges with on-grip and off-grip flags, and the first class assignment).
    - The contact graph is A4's method, built in Rust: `c600-jumble`'s `observe::seam_cells` gives the exact seam cells, and `c600-display`'s `jumble::contacts` runs the binary64 separating-axis test, since CT-KN2-05 keeps floats out of `c600-jumble`. The binary `c600-jumble-contacts` checks each record's digest.
    - At wj-ia-300 it gives A4's counts: 54 contacts, 20 with the lattice domain, 707 planes and 98,048 seam faces; four classes there and at the deep exports, one at ia-40.
  - Model (`Model/JumbleTint.cs`): `TintMode`, one setting `tint.mode`, so modes never stack. While it is unset the entry slot's armed `tint.contact_class` is the mode, so contact classes are on at open.
    - `ContactClasses.Assign` keeps a remembered class unless a touching domain, in index order, already kept it, then colours the rest by DSatur with six classes and the overflow state; the lattice domain is neutral.
    - The memory keeps the first class each domain id was given (overflow never), so an undo, a redo, a branch switch or a reopen draws a state as it did. It is saved as view-state text in the chrome setting `tint.contact_class.memory`.
    - The core's palette: 1 and 2 the selection and the hover, 3 to 8 classes 1 to 6, 9 the overflow, 10 to 13 the height steps 2, 4, 6 and above 6 (alpha 0.6 on the fill, so the label shows), 14 off-lattice.
  - Host (`Ui/JumbleLegend.cs`, a part of `JumbleHost`): the palette row `jumble.tint.mode` (CR-14's draft row; a mode, or without one the next) and the tint legend with counts on Global's and Local's borders. `jumble.focus.domain` now dims the other domains at once, as its draft row says; height and straddler focus still dim on F10.
  - Shell (`D02/ShellTint.cs`, `D02/ShellStyle.cs`): at each binding the host gets the contact graph and pose ids. Each change sends per piece the classes or the off-lattice mark on the outline channel under the selection and hover outlines, and the height ramp on the fill with the focus dim bit. Both views take the same bytes (slots 1 and 2), so Local draws Global's tint for its cap; each slot dims only while its own view's focus layers dim. `FocusLayers.Changed` now reaches `ShellStyle`, which B17 part 2 left to the integrator.
  - Readings, for the owner:
    - Off-lattice has no design slot. It takes the darker of `surface.window` and `text.primary`; a slot `tint.jumble.off_lattice` is requested.
    - The overflow's "double outline" cue is drawn as its slot's colour only.
  - Reused: A4's contact method and `observe.rs` (seam scan, domains, pole of a face); `faces.rs`'s piece faces; B11's `PoseStyle`, `FocusLayers`, `JumbleFocus` and the Focus layer's `FilterExpr`; B14's style path and B17's `ShellStyle`; `ReadoutRow`'s border pattern for the legend.
  - Checks: `check.py`'s console lane passed 246 of 246 with the eight named tests; `test_shared_chrome.py` (12 OK), `build_product.py` (PASS), `test_renderer_shell.py` (95 OK), `test_d02_drawing.py` (45 OK). `d02_run.py --script tint --state wj-ia-300` passed: in seven steps the bytes sent on both channels equal the sample's piece by piece. `two-views` on wj-ia-300 passed in sample mode, with 1,195 of 1,203 frames drawn with two panes and none held, so the sample-mode collapse does not show.
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
  - The hidden-piece masks per named cap, added to `make_d02_samples.py` here from B1.
- **Check:** `check_core.py --lines` on WARP: a line layer covers pixels only within two pixels of its projected segments; an empty layer changes nothing. Then `d02_run.py --script overlays`: hidden count equals the sample; the never-hidden set stays visible under every focus layer.
- **As built, drawing track (B16, 26 November 2026):**
  - The overlays reuse the core's one line drawing, the outline channel of B14's style path, as the integrator's brief asked, so there is no `sa2_lines_set`, `lines.hlsl` or `check_core.py --lines`, and the core, the bridge and the host are unchanged. The overlay takes palette entry 15, the one the tint's plan leaves free, over the tint and under the selection's and the hover's outlines.
  - Chrome (`Model/JumbleOverlay.cs`; `Ui/JumbleOverlays.cs`, a part of `JumbleHost`): while a cap is named (the default cap is not), its cut's trace at piece resolution: an admissible cap's inside set, whose border is the cut, or a blocked cap's straddlers, which lie across it. A focused repair row's step k outlines that step's moving set in place of the trace (ADR-D02.34). Rule 5's never-hidden set (a blocked named cap's straddlers, the focused step's moving set) and every overlay piece break through every focus layer, and the overlay pieces a focus layer would hide are counted exactly. Certificate-point glyphs stay off; a blocked cap's line counts its certificate points in their place. The overlay's line shows on Global's and Local's borders (`ViewLine`).
  - Shell (`D02/ShellTint.cs`, `D02/ShellStyle.cs`): the overlay's bytes go on the outline channel with the tint's, `SetFocusViews` takes the breakthrough, and the dump's `overlay` object names the layer, the counts, the line on each border, the pieces sent with entry 15 and the kept pieces that still carry the dim bit.
  - Readings, for the owner:
    - The trace is drawn as outlined pieces, not as line segments in four dimensions. Constant line data per cap needs the core line layer planned above (`sa2_lines_set`), which stays a candidate.
    - A hidden piece that matters shows in full with its outline, not as an outline alone: the core has no outline-only draw, and a style bit for one is requested.
    - The overlay's colour is `state.preview`; a slot `line.jumble.overlay` is requested.
    - No hidden-piece masks were added to `make_d02_samples.py`: the exact count comes from the cap records' masks and the focus layer at run time. Protection conflicts (rule 5, item 3) do not arise on the Jumble sample states.
  - Checks: `check.py`'s console lane passed 252 of 252 with the four `D02Overlay*` tests; `test_shared_chrome.py` (12 OK), `build_product.py` (PASS), `test_renderer_shell.py` (95 OK), `test_d02_drawing.py` (51 OK, with `OverlaysRun`), `tools/wiki/lint.py` (ok). `d02_run.py --script overlays --state wj-ia-300` passed at 1280x720: in six steps the overlay's counts, its line on both borders and the outline and fill bytes equal the sample's piece by piece, and under the domain focus, dimmed and then hidden by F10, no overlay or never-hidden piece carries the dim bit.
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
- **As built, part 1 (drawing track, 26 November 2026):** picks, with ADR-D01.41's shared selection and hover.
  - c600-render's bridge stamps every slot of the posed row in the ID pass as its home slot + 1. The plan still carries no instance of a posed primitive (J13).
  - The host's version 14 adds `m6h_pick_request` (surface pixel, newest frame; in a composite frame the pane under the point answers) and `m6h_pick_poll`.
    - A hit resolves through `jumble::PoseProjection` to the sticker's piece, the slot it occupies (none off the lattice) and the lattice flag.
    - A hit whose frame drew a revision the head has left is dropped as stale, as is one the core finds stale.
  - The shell (`D02/ShellPick.cs`) keeps one selection and one hover for both views.
    - A click that did not turn the camera selects. A click on no sticker clears the selection.
    - The pointer hovers, with at most one hover pick in flight, one every 80 ms.
  - `D02/ShellStyle.cs` outlines the selection (the core's marked colour) and the hover through B14's style path. It sends them again for each drawn revision. Global's pane uses style slot 1 and Local's slot 2.
  - Reused: c600-display's `PoseProjection` (J13) and the core's ID pass; nothing new in the core.
  - Waiting on the integrator (`ShellJumble.cs`):
    - the script's `click` step, through `ShellPick.ScriptClick`;
    - the dump's pick context, `ShellPick.Dump`;
    - the inspector's tier 1 line and the pick-and-key naming of a cap (`ShellPick.Selection.Cell`).
  - Checks:
    - Host tests `a_jumble_pick_resolves_to_its_piece` and `a_stale_jumble_pick_is_dropped` passed; the first compares the slot with the projection's. All 23 turns tests passed.
    - `cargo test -p c600-render`, the bridge workspace and `test_render_ffi.py` passed.
    - `tests/test_d02_drawing.py` passed 37 of 37 and `test_renderer_shell.py` 84 of 84.
    - With the outlines on, `d02_run.py --script hopf` and `--script two-views` (632 two-pane frames) passed on `wj-ia-300`.
    - `--script pick` waits for the `click` step.
- **As built, part 2 (drawing track, 26 November 2026):** ADR-D01.46's focus layers drawn in the core, and Local re-framed on the named cap (the drawing half of .43). Added to ABI 10 and host 14 before any release.
  - The core:
    - A style slot's `dim_mode` may now hide what it would dim: `SA2_STYLE_HIDE_MARKED` (3) and `SA2_STYLE_HIDE_UNMARKED` (4). `styledVs` places a hidden piece's stickers outside the clip volume, so what lies behind them shows.
    - Picking follows visibility. The ID pass of a styled frame, or of the styled pane under the point, draws with that frame's or pane's style through the new `styledIdPs`. A dimmed piece answers no instance, and a hidden one is not drawn at all.
    - A pick on a styled frame whose engine copy has since taken another piece style is `SA2_PICK_STALE_ENGINE`. A frame drawn without a style picks as before.
  - The shell:
    - `D02/ShellStyle.cs`: `SetFocus(layers, member, breaksThrough)` marks the pieces the focused predicate leaves out (the fill byte's dim bit). The focus layer's inactive mode chooses dimmed (0.75) or hidden for both views.
    - `D02/ShellLocal.cs`: `Frame(namedCap)` re-frames Local on the cap the solver named. The shared selection and hover (.41) stay as in part 1.
  - Reused: B14's style path and the core's ID pass. The pick rule follows the style the frame drew with, as the product records already did for shapes.
  - Not built on this track:
    - .43's other half, the .6cs caret as the preview's focus source, belongs to the editor.
    - .67 (the Target by a pick and a key) needs the integrator's key binding of `ShellPick.Selection.Cell`.
    - `FocusLayers.Changed` reaching `ShellStyle.SetFocus` is one line in `ShellJumble.cs`, sent to the integrator.
  - Checks:
    - `check_core.py --style --hopf --panes` passed. The CPU checks passed 40 of 40, and a CPU check refuses `dim_mode` 5.
    - On WARP, hiding the marked pieces covered 26,162 pixels with 0 apart from a set without them. 7 of 8 hidden sample points picked the piece behind. The dimming frame's picks never named a dimmed piece, and an outline alone changed no pick.
    - `tests/test_d02_drawing.py` passed 40 of 40, `test_renderer_core.py` 26 of 26 and `test_renderer_shell.py` 84 of 84.
    - `d02_run.py --script hopf` (316 frames) and `--script two-views` (633 two-pane frames) passed on `wj-ia-300`.

- **As built, part 3 (drawing track, 27 November 2026):** the pick script hits its pieces, the base puzzle picks, and the inspector and the Target follow the pick (ADR-D01.67).
  - The second cause of the pick misses was pixel choice, not the ID pass:
    - A temporary core trace, removed before commit, showed every click of `--script pick` running the ID pass on Global's pane: pane 0, view 1, resource set 3, style slot 1, no dimming, no slice hole.
    - A full-pane ID mask showed that Global's opening layout, the Hopf rings, covers 7,110 of the pane's 118,146 pixels (about 6%). The script's fixed click fractions mostly fall between stickers.
    - The one hit named the piece the model gives (sticker 233857, piece 137602). The note in part 2's work that pixel choice was ruled out was wrong. The first cause, the core dropping a posed row's instances (F02), was fixed in f620475f.
  - Codex call `20261127T005255Z-8c7e2986` (gpt-6.2-sol, effort max; wrapper exit 0, acceptance 293 of 293) was reviewed and applied with one change:
    - A pick is refused as unavailable until the frame of the engine's current revision has its pick resources, and `ShellPick` keeps the click until then. This closes a startup gap in the one-view path.
    - The base puzzle picks. `RenderBackend::with_retained_sticker_ids()` stamps each base slot with its sticker's label + 1, and `resolve_sticker` reads it at the publication's base state. The fake core applies F02 as the real one does.
    - The inspector's tier-1 line, `PieceInspection`, gives the selection's piece, sticker, cell, sample, domain and height.
    - E names the selected cell as the Target (ADR-D01.67). E already maps to `target.current.set`, so no draft row was needed.
    - Changed: the patch named the publication's base state for both puzzles. The Jumble arm keeps part 2's subject, the snapshot of the revision the frame drew.
  - Scripts:
    - `pick.json` adds three clicks on sticker blocks of the Hopf rings at the run's 582x203 opening, the last on a lattice piece, and then E.
    - `base-local.json` and `d02_run.py` click the corner first, which now hits a sticker of the base view. The clearing click moves to the opening's centre, where the opening state draws no sticker.
  - Item 6 landed in 34b58afd. The inspector's Focus layer editor stays, since ADR-D01.46's focus layers serve both puzzles.
  - Runs (build 20261127T012852Z; no nvlddmkm event or LiveKernelEvent in the System log after any run):
    - `--script pick` passed: 11 clicks, 4 hits, none stale. E named cell 106, and the inspector read "piece 33859 · sticker 46015 · cell 106 · sample · domain 22 · height 0".
    - `--script slice` passed (947 composite frames, 109 with three panes). `--script base-local` passed (1 of 3 clicks hit; Local framed cells 0 and 402).
    - `--script two-views` failed: after F1 and `> jumble.cap.name 150`, no cap was named and no revision changed while Local was drawn. Builds of f620475f and 34b58afd fail the same way, so the failure predates this part. It is left open.
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

**As built (B18 part, for B20's turn preview; 26 November 2026, branch `claude/d02-drawing`).** Only what B20 needs was built; the sweep itself waits.
- Built:
  - Core: `sa2_transform_set(ctx, revision, plane_u, plane_v, angle, moving_mask, phase)`, added to ABI 10 before any release (no new number). Moving pieces draw turned by R((phase - 1) angle) after their pose, in frames, panes and picks. Phase 1 is byte-identical to no transform; phase 0 is the pose before the twist.
  - The moving bit is bit 16 of the per-piece style word, so the mask shares the style upload. Adoption drops the transform; `sa2_recover` keeps it.
  - A preview of an uncommitted twist passes the negated angle: its phase 1 - s draws the fraction s of the twist, and phase 0 draws the whole twist.
  - Bridge and host: `m6h_set_transform` (host 13, no new number) and `m6h_twist_transform(cap, item)`. The latter returns the certified row (`c600_display::jumble::transform`) of the session's twist without committing it, or status 0 when the twist is not admissible or is the identity (menu item 0).
  - Shell: `HostLibrary.SetTransform` and `TwistTransform`. `D02/ShellTurn.cs` reads item 1's row at the cap Local draws, once per cap and revision, and the dump writes it as `turn`.
- Readings:
  - The focus test and the Hopf chart use the unturned centre (a cap twist keeps dot(c, pole)).
  - "Phase 0 equals the old pose" holds to within 1% of covered pixels on WARP: the turned piece is the new pose rotated back in floating point, not the old revision's bytes. Phase 1 is byte-identical.
- Checks:
  - Host test `a_committed_jumble_twist_carries_its_certified_transform`.
  - `check_core.py --sweep` on WARP: 26,304 covered pixels; phase 0 differs from revision 1 at 4, and phase 0.5 differs from both revisions.
  - `d02_run.py --script twist-row --state wj-ia-300`: the default cap 0 (blocked) has no row. Cap 150's row has the angle pi and moves exactly its inside set of 1,909 pieces (the sample mask's sha256).
- Waits:
  - The timed sweep, with its duration from the row `jumble.sweep` (`Model/Gallery.cs`, `MotionTable`).
  - The half-turn's cut and outline flash.
  - Reduced motion and the refused twist.
  - `d02_run.py --script sweep`.
- Owner sees: nothing new on screen yet. The run's `turn` dump shows the row that B20's ghost will draw.
- Review (gpt-6.2-sol, call 20261126T225307Z-e6ce1514, of `git diff f9eb87a0 8654dbe2`; dispositions, 27 November 2026):
  - B18-01 (blocker), fixed in 1eec73b4: `HostLibrary.SetTransform` refuses a plane that is not 8 floats before the native call. `m6h_set_transform` has no plane length in its ABI, so it reads the plane only once `moving` holds one byte per piece. Checked by `TwistRowShell` in `tests/test_d02_drawing.py` and the host test `the_host_sets_the_piece_style_and_the_style_slots`.
  - B18-02 (major), fixed in 1eec73b4: the row protects the pieces that carry the protected orbits, as the command layer does for a pose move (read only). For one twist, Net and Strict give the same verdict. A twist the layer refuses now has status 0. Host test `a_twist_that_moves_a_protected_orbit_has_no_transform_row`; it fails on the old `turns.rs`.
  - B18-03 (major), fixed in 1eec73b4: a pick on a turning frame, styled or not, checks its copy's style version. A frame whose copy now holds another mask answers `SA2_PICK_STALE_ENGINE` instead of picking through that mask. WARP check `transform picks warp` (four unstyled turning frames, four masks); it fails on the old `product.cpp`.
  - B18-04 (major), fixed in 1eec73b4: `m6h_twist_transform` returns the row's revision, which is the engine revision handed to the head. `m6h_set_transform` takes that revision and refuses any revision but the adopted one. The host test covers the setter's refusal, not a held adoption in a live session. Still open: B20's `ShellTurn.SendInside` sends the row's moving set through `m6h_set_piece_style`, which carries no revision, so the slice's inside layer can still draw for one frame on the revision before the row's.
  - B18-05 (major), fixed in 1eec73b4: `ShellTurn` reads `ShellLocal.Plan.Framed`, the result of the shell's one framing call per frame (FollowLocalCap). Only a source check covers it (one `ShellLocal.Frame(` call in the project). The product scenario (name cap 1, then select in another cell) was not scripted.
  - B18-06 (minor), fixed in 1eec73b4: a refusal is kept for its cap and revision, like a row.
  - B18-07 (minor), fixed in 1eec73b4: the host checks a moving byte, the plane (finite, unit and orthogonal to the core's 1e-3), the angle and the phase. Each answers `M6H_E_INVALID_ARGUMENT` before the core is asked.
  - B18-08 (minor), fixed in 1eec73b4: `expect_twist_row` pins cap 0's status to exactly 0 (blocked), cap 150's angle to pi and its plane's sha256 (`c7b19b2e…`), and the head digest before and after the query. The `turn` dump now writes `revision`, `read_at` and `plane_sha256`.

**As built (B18 rest; 27 November 2026, branch `claude/d02-drawing`).** The sweep, the half-turn flash, reduced motion and the refused twist, on B18 part's transform.
- Built:
  - Shell (`D02/ShellTurn.cs`): before a key-down, a twist key's certified row is read at the router's grip cap with `TwistTransform`. The grip cap is cap 0, and the key table mirrors c600-command's draft `TWIST_KEYS`; a test pins both to `keymap.rs`. A committed press that entered the revision after the row's starts a motion. A half-turn (angle pi) flashes and any other twist sweeps. A refused press, a twist with no row and reduced motion start none.
  - The sweep: `SetTransform` with the row's revision, plane, angle and moving set. Phase 0 shows on the revision's first frame. From the next step, the phase follows `MotionTable.Ease` of the `jumble.sweep` row's curve over its duration (token `twist`, 220 ms) and ends at 1. The next adoption drops the transform, as before.
  - Host (`m6h_set_transform`, host 13, no new number): a revision handed out but not adopted yet is held and set when it is adopted. The first product run without this drew the end pose for one frame before phase 0 (the dump's `popped`). Host test `a_transform_named_for_a_pending_revision_is_held_until_its_adoption` (`tests/sweep.rs`) fails on the old setter. A revision not handed out is still refused (B18-04).
  - The flash: the moved set is outlined in palette entry 15 (B16's overlay line, `JumbleOverlay.Entry`) on the outline channel, under the selection and the hover, for the `jumble.half_turn` row's time (token `flash`, 320 ms).
  - Reduced motion: the existing setting (`Appearance.ReducedMotion`; the Appearance panel's check box, palette row `motion.reduced.set`), read through `MotionTable.Seconds`. Its 0 seconds make both motions static: no transform and no outline.
  - Dump: a `sweep` row with the refused count, the reduced flag and each motion's kind, revision, angle, moving count, duration, end, `popped`, times and phases.
  - `d02_run.py --script sweep` with `expect_sweep`. The script names cap 1 with Space, clicks three lattice pieces, then presses J, K, reduced motion on, K and J.
- Readings:
  - Twist keys turn only the grip cap 0, and wj-ia-300 blocks cap 0. On wj-ia-300 the four presses are refused and nothing moves. The committed sweep and flash run on the solved jumbled puzzle (`--script sweep` without a state). A twist at the named cap waits for `grip.select` (c600-command and c600-input; a row request).
  - The motion table's static equivalent ("the moved set outlined") is read as no outline under reduced motion, as the brief says. A lasting static outline would be a new mark and is not built.
  - The first step after the revision's first frame comes about 60 to 80 ms later in these runs. The sweep's time starts at that step, so the whole sweep is drawn rather than its first half skipped.
- Review leftovers:
  - B18-04, closed. `ShellTurn.SendInside(library, host, adopted, out count)` sends the row's moving set only while the row's revision is the adopted one. Otherwise it answers `Unavailable` and sends nothing. The slice keeps the layer sent for the adopted revision and tries again at the next step. Shell test `test_the_inside_set_is_bound_to_the_rows_revision` fails on the old `SendInside`.
  - B18-05, closed. The product scenario is scripted in `sweep` on wj-ia-300: name cap 1, then three clicks on lattice pieces with auto-framing on. Local frames each selected sticker's cell (578, 94 and 149), and the `turn` row reads the same cell. pick.json's three hits are off-lattice pieces, which have no cell, so Local keeps the named cap there. The lattice points were found by a grid of 240 clicks at 1280x720.
- Checks: listed in the commit message. Product runs at 1280x720 (two-views at its own 2000x1200): `sweep` with and without a state, `twist-row`, `slice`, `two-views` and `pick`, with no nvlddmkm or LiveKernelEvent event in the System log.
- Owner sees: on the solved jumbled puzzle, K sweeps cap 0's inside set over 220 ms, and J flashes the moved set's outline for 320 ms. With Reduced motion on (Appearance, or `motion.reduced.set on` in the palette) both cut at once. On wj-ia-300, J, K and L are refused and nothing moves.

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
- **As built, drawing track (integrator, 26 November 2026; merged as c75dd35a):**
  - Core ABI 8. `sa2_frame_submit_panes` takes one to four 112-byte panes: view, view generation, resource set, rectangle, camera, style slot and flags.
  - It draws one clear, one draw per pane with its own viewport and scissor, one present, one frame ID and one report. Rectangles must lie inside the target and not overlap, and a pane prepared at another revision refuses the whole frame. `sa2_frame_panes` returns each pane's vertex evaluations, and they add up to the report's.
  - A pick on a composite frame names the pane's view. A point in no pane is refused.
  - Departure: each view keeps its own publication generation, so only the first pane must carry the frame's generation.
  - Checks: `check_core.py --panes --style --shape` passed with 40 CPU checks on the drawing track:
    - one full-size pane is byte-identical to `sa2_frame_submit`;
    - two 300×300 panes match their one-view frames (14 of 36,819 covered pixels differ by more than 2);
    - no pixel changes outside the panes;
    - the vertex evaluations add up: 36,576,000 = 18,288,000 + 18,288,000.
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
- **As built, drawing track (integrator, 26 November 2026; merged as c75dd35a):**
  - Host 11 adds `m6h_open_local`, `m6h_set_view_camera`, `m6h_set_view_visible` (Local only; Global is never hidden), `m6h_set_pane` and `m6h_clear_pane`. Each adoption publishes both views from one bundle, and recovery republishes Local.
  - Departure: the framework says a frame "re-submits the last complete pane list", but the core holds only the adopted revision's image. So while a drawn pane's view lags the adopted revision, `m6h_frame` submits nothing, sets `held = 1`, and names the last frame, which stays on the surface. `M6hFrame` grows from 40 to 48 bytes (`panes`, `held`), and the shell's struct matches.
  - Nothing in the shell calls the two-view exports yet: B18c opens Local, and it must also set Local's shape, since `ShellShape.Apply` sets only Global.
  - For B17's picks: a host export next to `m6h_frame` that answers which view's pane holds a surface point, and the pane list back in `FrameRecord`.
  - Checks: `test_render_ffi.py` passed on the drawing track, including six new tests: `two_views_publish_one_revision`, `a_frame_never_mixes_revisions`, `a_hidden_local_adds_no_vertex_evaluations`, `one_view_frames_are_unchanged`, `a_fifth_pane_is_refused`, and the composite frame crossing the table. After the merge, `test_render_ffi.py`, `test_renderer_core.py` 24 of 24, `test_renderer_acceptance.py` 54 of 54 and `test_renderer_shell.py` 71 of 71 passed.
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
- **As built, part 1 (integrator, 26 November 2026):**
  - `Model/ViewPanes.cs`, pure:
    - `PaneRect` is a rectangle in whole pixels, and `PaneSpec` a view with its rectangle.
    - `Layout(openings)` gives the surface (the openings' bounding box in the window) and each pane at its offset within it. It refuses no opening, a fifth, an empty one, and any two that overlap.
    - `Split(opening)` gives two side-by-side panes of one view: the left takes the floor of half the width.
    - `Hit(layout, x, y)` gives the view and the point in surface pixels, and nothing between or outside the panes. `Takes(clicked, replied)` drops a pick reply that names another view.
  - Checks: console tests `D02PaneLayoutIsBoundingBox`, `D02PanesNeverOverlap`, `D02FifthPaneRefused` and `D02HitResolvesItsView` pass; the lane passed 203 of 203.
  - Not built yet: the shell part (Local as a second opening, the surface over the bounding box, the panes through `m6h_set_pane`, picks per view, Local's shape and the event log's `panes`) and the `two-views` script.
- **As built, part 2 (drawing track, 26 November 2026; a7c39e3d):**
  - Local's own shape needs the core, so B18c adds core ABI 9 and host 12:
    - `sa2_view_focus` (24 bytes: projection, focus cell, `focus_cos`, spread) and `sa2_view_focus_set`; host `m6h_set_view_focus`. The default focus (perspective, none, -1, 1) draws byte-identically to ABI 8, and a later shape keeps the focus.
    - With a focus cell the core turns the cell's unit centre to +w before the view's camera (Look Lab's `LocalShape.Frame`), drops d4's divide when orthographic, and moves each drawn sticker by (spread - 1) times its posed cell centre's projection. The constants grow from 176 to 208 bytes; W-J's two pad words become the flags and the spread, so every other path draws as before.
    - Departure: Local's region is a geometric cap test in the vertex stage, not an uploaded mask. A sticker is drawn when its posed, unshrunk centre has a dot product of at least 121/125 with the pole, which is the jumbled cap's threshold κ = (121/125)|n|² with the pole at cell c's centre. So Local follows the turning region through every twist without a per-revision upload.
  - Shell (with `--d02` only): Local opens beside Global, hidden, with `local-view.json`'s shape through `ShellShape.Set`, so the wheel scales Local's own zoom. It follows the named cap (`Naming.Named`): the focus moves to the cap, Local is shown and the chrome opens its slot. With no cap, Local is hidden and its empty sentence keeps the place. The surface covers both openings' bounding box. The pane table (Global in pane 0, Local in pane 1) is set once the surface has its new rectangle and viewport, and Local's zoom follows its pane's aspect. Local turns with the linked 3D spin. `ViewAt` resolves Local through `ViewPanes.Hit`, so the wheel zooms the view under the pointer. The `frame` row gains `panes` and `held` for composite frames only.
  - Chrome: Local's slot holds the drawn opening on top, a square of the slot's width capped at 60% of its height (`ViewPanes.OpeningHeight`), with Local's sections in a scroll container below it, so an open section never covers the opening. `GroundMask` cuts both openings (`ViewPanes.Ground`).
  - Script `two-views` on `wj-ia-300`: it names cap 150, twists twice with the router's KeyJ, and undoes. Its event check needs one-view frames before the cap, then composite frames of two drawn panes at every revision reached. The jumbled fixture refuses the two KeyJ twists (the run's correctness detail counts 2 refused), so only the undo changes the revision; the check asks for at least one change.
  - Not built yet: picks per view (B17; `Takes` is ready), camera keys to the focused view, and the `M600_FRAME_PICTURE` transparency check. Local looks for its cap with no `watch_d02.py` sitting yet.
  - Checks: `check_core.py --poses --shape --style --panes --focus` (CPU and WARP) passed. The WARP focus check covered 88,732 pixels for the whole view, 8,129 for the cap, centred on the target, and 9,965 with the spread. `d02_run.py --script two-views --state wj-ia-300` passed: 896 frames, 633 drawn with two panes, 0 held. `tests/test_d02_drawing.py` passed 20 of 20, `test_renderer_shell.py` 79 of 79, `test_renderer_core.py` 25 of 25 and `test_renderer_acceptance.py` 54 of 54. `cargo test -p c600-render` and `test_render_ffi.py` passed, including the host test `the_host_sets_a_view_focus`. The Look Lab console lane passed 204 of 204, including `D02GroundCutsBothOpenings`. Its Godot lane does not build: CS0108 in `Chrome/Ui/JumbleLists.cs` (the `Show()` methods of `CapCoversList`, `RepairListControl` and `RepairStageBar`), which B18c did not touch.
- **As built, the default cap (bug track, 26 November 2026):** the owner decided that Local is never empty.
  - The rule is the pure model `Model/LocalCap.cs`. While a state is bound and no cap has been named in the session, Local draws cap 0, the lowest cap id; the choice is neutral by number, as every cap list is in cap-id order. Once a cap is named, Local keeps it until another is named, also across undo, twists and an un-naming. With no state bound Local draws nothing, and its empty sentence stays. Console test `D02LocalCapDrawsTheLowestCapUntilOneIsNamed`.
  - The default is drawn, never named. This follows ADR-D02.12 ("Readouts show true zeros"), .16 ("No job starts before a cap is named") and .19 ("A cap's cell names the cap"). The Named cap count, the cap map's "no cap named" line, the threshold panel and the jobs read `Naming.Named` only. Local's readout says "Local shows cap 0 by default; no cap is named." followed by the empty sentence as the hint, which its tooltip repeats. ADR-D02.12's "Local shows its empty state" still needs the integrator's wording.
  - Shell: `ShellLocal.Frame` falls back to the host's `LocalCap.Shown` while no cap is named, and the script player tells the host whether a state is bound. `Shell.cs` is unchanged; its comment "with no cap Local is hidden" now holds only while no state is bound.
  - Scripts: `host` and `name-cap` expect the default's readout before the naming. `two-views` expects Local framed on cap 0 with its opening shown before the naming, and on cap 150 after it. Its event check, and those of `hopf` and `ring-sheet`, ask that no one-view frame follows the first composite frame.
- Checks: the Look Lab lane passed 234 of 234, including `D02LocalCapDrawsTheLowestCapUntilOneIsNamed`, with the checkout unchanged. `tests/test_shared_chrome.py` passed 12, `tests/test_renderer_shell.py` 94 and `tests/test_d02_drawing.py` 41. `build_product.py` passed. These `d02_run.py` runs passed: `name-cap` on `ia-40` (Local on cap 0 before the naming, then on cap 30), `two-views` on `wj-ia-300` (cap 0, then cap 150 through the twists and the undo; 883 frames, 875 drawn with two panes, 0 held), `host` on `ia-40`, and `hopf` and `ring-sheet` on `wj-ia-300` (307 frames each, 299 composite).
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
- **As built, gate tools (B19, 27 November 2026): tools built, sittings not run.**
  - `tools/perf/d02/gate_row.py` prints the `D02_GATE` line from `events.jsonl`. It times the gaps between the confirmed
    presentations (`presented` rows, `display_qpc`) over the last 60 s and drops everything before them as warm-up (the
    launch, the open, the view setup and the first 10 s of motion). A missing required field refuses the row, and the
    tests are in `tests/test_gate_row.py`.
  - `view_vertex_evals` is the core's own count: on a gate run (`M600_D02_GATE=1`), every frame row carries
    `sa2_counters` (`CoreSurface.GateFields`, called from `EventLog.Frame`).
  - VRAM is the shell process's dedicated GPU memory, sampled from outside through PDH's GPU Process Memory counters.
  - `renderer/shell/gate_d02.py --row <id>` runs a row through `d02_run.py`'s launcher and player, with the scripts
    `d02_scripts/gate-g*.json`. Their one repeat block is unrolled by `d02_run.expand_repeat` as quiet steps, and the
    player writes no screen dump after a quiet step (`ShellJumble.cs`).
  - The page is `docs/progress/1.0/adr-d02-gate-results.md`, with the rows, the commands and empty result cells.
  - Found while building, and on that page:
    - G3's S1 is no menu letter, so the fallback is `wj-ia-300`'s cap 0 with 462 certificate points.
    - The twist keys turn the constant grip cap 0, which is blocked at `deep-2`, so the sweeping rows need
      `grip.select` or a twist at the named cap.
    - Local cannot be hidden while a state is bound, so G1 has no one-view case yet.
    - The never-foreground window presented at a median 33.3 ms in the dry run.
  - The dry run (G1 at `wj-ia-300`, 10 s) printed a well-formed line and was invalid, as expected: two panes and no
    committed twist. It is not gate evidence.
  - Fixed the same day (gate track):
    - The sweeping rows select cap 2 with `grip.select` before the repeat block, and the router's twist keys turn the
      selected cap (`set_grip`, `ShellTurn`'s active grip cap). `gate_d02.py` moves the cap to an admissible one of a
      dry run's state from its sample `rim.json` (cap 91 at `wj-ia-300`) and records it.
    - `view.local` hides and shows Local (`ViewPanes.LocalOpen`, `LocalHide`): Local's opening closes, the pane drops,
      the host stops drawing Local, and Local returns on its cap with the camera shared. The key L stays the router's
      twist key in a live session. Tests: the console test `D02LocalHideDropsAndRestoresLocalPane` and the product
      script `local-hide`.
    - The window is the display's native size read at run time (2560x1600 here); `--resolution` overrides it, and
      `--foreground` (the owner's sitting only) activates the window once, never topmost. `gate.json` records both.
    - Dry runs at `wj-ia-300` (10 s, not evidence): G1 was valid with one pane and 7 twists, G9 valid with two panes and
      7 twists. The System log showed no GPU event after either.
  - The twist stall (SF, 27 November 2026). The dry runs showed a 300-480 ms frame at each twist, all of it on the
    frame thread. Per step, before and after (release host, `wj-ia-300`):
    - `m6h_twist_transform`, called twice per twist (the press's row, the cut slice's row): 66-72 ms, of which
      `Menu::from_record` was 58-64 ms, then 0.07-0.24 ms. The session keeps the layer's pinned menu, and its rows per
      head and (cap, item), which a worker reads ahead after each commit. A row never asked before still costs
      `State::apply`'s admissibility check (7-12 ms). Tests: `renderer/bridge/host/tests/key_timing.rs`.
    - The binding change at a twist (13 of 20 presses): the bind chip's `frame.Invalidate()` (a full `ChromeFrame.Build`)
      and the per-state JSON reads, 160-560 ms; now no rebuild (the sweep script checks the rebuild count) and each
      state file parsed once. What is left is the bind's own in-place updates, p50 56 ms and p99 97 ms in G1
      (`SetStyle`'s filter model, `SetCapMap`, `SetNu`).
    - `m6h_key_down`, 39-48 ms: `Turns::key_down` is 34 ms in the bench (the command layer's commit, its journal write
      and its digests, which stay as they are: ADR-D01.55, KN5), and the head's copy is 0.04 ms. In the product it is
      p50 46 ms. After it, `ShellChrome.Committed` (the History row) is p50 25 ms and p99 38 ms.
    - The shell writes these steps as `steps` rows while the event log is on (`EventLog.Steps`).
    - Dry runs (10 s, not evidence): G1 p99 433 ms before and 117 ms after (fps_avg 39.4, then 48.7); G9 p99 300 ms
      before and 217 ms after (fps_avg 39.0, then 46.4). The System log showed no GPU event after either.
    - SF2 (27 November 2026), `m6h_key_down` off the frame thread: the shell posts each key to the host's ordered key
      queue (`m6h_key_post`, host ABI 15) and keeps drawing. The host's one key worker takes the session's `Turns` and
      routes the waiting keys in press order as the synchronous call does (the armed row first, at the head the key will
      twist); the frame thread applies each answer at its next frame (`m6h_key_poll`: the deferred adoption, the row,
      the History row, the style), so nothing of a key shows before its commit. A call that reads the session waits for
      the batch; one that writes it (a row, a job, a grip change, a close) routes the waiting keys first; `m6h_key_drain`
      and `m6h_close_session` drain the queue (the shell has no reset or import yet). A sent twist key holds the
      motion queue as a running sweep does (`MotionQueue.Waiting`, none under reduced motion), and a script's dump
      waits for the posted keys' answers. Test: `key_timing.rs`
      `posted_keys_end_where_synchronous_keys_end_at_the_wj_ia_300_head`: posted keys with a refused key in the middle
      give the synchronous answers, head digests, objects and journal records (the session's random command epoch and
      the request digests that hash it apart); a close drains. The post call: max 0.07 ms on the caller's thread, where
      `m6h_key_down` was p50 46 ms. G1 steps after: `key.post` p50 0.0, p99 1.1 ms; `key.queued` (the commit, on the worker while frames go on) p50 67, p99 80 ms; still on the frame thread: `bind.bind` p50 62, p99 96 ms, and the History row, `key.chrome` p50 25, p99 42 ms. All ten product runs, J1 and both dry runs: no GPU event in the System log.
    - Dry runs (10 s, not evidence): G1 p99 117 ms before and 100 ms after (fps_avg 48.7, then 49.8); G9 p99 217 ms before and 133 ms after (fps_avg 46.4, then 49.0). The bind's in-place updates and the History row (SF2 items 2
      and 3) are open and are what the slow frames are made of.
    - SF3 (27 November 2026), the bind's updates and the History row: the bind holds the host (`JumbleHost.Hold`) so
      its border rows are placed and Local's sections refreshed once, at the bind's end (`flush`), not at each `Set*`;
      a `ReadoutRow` of the same shape updates its changed slots in place; the cap record's text is kept per state
      (`Kept`). The History list adds the committed row (`HistoryPanel.Update`: the next history revision, its last
      action that commit, at the end of an unbranched line, no question open), and anything else rebuilds it as before,
      once per change instead of twice. The `bind` and `key` steps rows now name their parts; a new `shown` row times
      the chrome at a newly shown revision. Tests: `tests/test_renderer_shell.py` `D02TwistStallSF3`. Per part, G1
      and G9, two runs each, p50 (p99), before and after: `bind.bind` 46-48 (78-103) and 16-19 (65-72; the largest is
      the first bind at startup, which parses the style and tint data, outside the gate window); `capmap` 27-29 and
      4-6; `feed` 7.6 and 2.2-2.4; `totals` 7.1 and 0.01; `flush` 3.4-3.9; `key.chrome` 18.6-19.1 (32.5-36.3) and
      3.6-4.0 (8.1-9.5), of which the History row is 0.00 and `describe` 1.7-1.9; `shown.describe` 2.7-3.1. All ten
      product runs, J1 and the four dry runs: no GPU event in the System log.
    - Dry runs (10 s, not evidence): G1 p99 83.34 and 83.34 ms before, 83.33 and 83.34 after (fps_avg 52.0 and 51.4,
      then 54.5 and 54.6); G9 p99 83.34 and 83.34 before, 83.33 and 83.33 after (fps_avg 50.9 and 50.9, then 54.6 and
      54.9). What is left is in the core's present path, not the frame thread: each twist gives one 83.3 ms presented
      interval (11-15 over 33.4 ms in the window), also where the bind is only `state_digest` (2-4 ms); through it the
      frame thread keeps its 16 ms cadence (C# steps under 4 ms, `m6h_frame` about 14 ms) while four submitted
      frames get no `presented` row.
    - SF4 (27 November 2026), the dropped presentations. A core trace (`core-frames.jsonl` beside `events.jsonl`,
      written only while the event log is on: each frame's parts and each statistics sample with the present
      ledger's state) ranks the hypotheses. 1, confirmed: the core's surface is resized at most twists. The Global
      opening's height alternates between 1093 and 1116 px, so the shell's surface follow (`Shell.cs`, the
      `surface.Resize` branch of the frame loop) drains and calls `sa2_resize` (`CoreSurface.Resize`), and the core's
      `resize_surface` (`renderer/core/src/interop.cpp`) calls `ResizeBuffers` and raises the surface generation. In
      one G1 run every one of the six 83.3 ms gaps in the window begins 23-24 ms before one of the window's six
      resizes, and the 50 ms gaps (frame-thread holes) have none. 2, a consequence: after the resize DXGI's
      statistics are disjoint and stay at one present count for five refreshes, then advance five presents in five
      refreshes, so no present was dropped or repeated; but the present ledger's exact rule (`present_match`,
      `renderer/core/src/product.cpp`) supersedes the four frames of the old generation, and they get no
      `presented` row. 3, ruled out: a slow core frame (G1: ready and upload waits 0.00 ms; the frame-latency wait
      p50 13.0, p99 13.8; recording 0.04 (0.25); present 0.26 (0.39)), and a frame-thread stall longer than the
      queue. The likely source of the height change is a border readout that rewraps when its numbers change at a
      twist (`TintLegendRow`'s `N0` counts or a `ReadoutRow`, in `tools/looklab/app/Chrome/Ui/`); not yet pinned.
      A reading fix was tried and is not committed: a ledger that confirms the frames between two statistics
      anchors n presents and n refreshes apart at a steady rate (core ABI 11) confirmed 7-14 frames per window but
      none across a resize, and the dry runs stayed at p99 83.33 (G1 83.33 and 83.33, fps_avg 57.0 and 57.2; G9
      83.33 and 83.33, 56.6 and 56.7; before it, G1 83.33, fps_avg 56.4); no GPU event in the System log after any
      run. Left: stop the per-twist resize where the opening's height changes (a readout's changing numbers must
      not change the opening, which also stops the view from moving 23 px at a twist), then rerun G1 and G9; the
      trace and the tried reading are kept outside the repository for that step.
    - SF5 (27 November 2026), the opening's height. A `surface` row in the event log at each resize of the core's
      surface, with the Global footer's rows and their heights (`Shell.cs`, `ShellChrome.FooterRows`), pinned the
      rows: at a K twist of G1 the state alternates between a sample state (wj-ia-300's cap-2 powers) and one no
      sample holds, where nothing is bound, so the tint legend (`JumbleLegend.cs` `Legends`, `TintLine` empty) and
      the overlay line (`JumbleOverlays.cs` `OverlayWords`, no cap record) were removed and added again, 23 px each:
      footer 134, 111 or 88 px, opening 1070, 1093 or 1116 px. It was not a rewrap. Now a shown legend row stays,
      blank at its height, while a mode is on, and the overlay line while a cap is named (`JumbleTint.LegendKept`,
      `BorderRowKept`, `ViewLine.Set(keep)`); only no mode or no named cap removes them. The legend's counts sit
      in slots as wide as their widest form (`JumbleTint.EntryParts`, `WidestCount`), so a count never rewraps
      the row. Tests: `D02LegendRowKeepsItsHeight` (console), `sweep`'s `global_opening` check (d02_run.py), and
      `gate_row.py`'s `surface_resizes`. Dry runs (10 s, wj-ia-300, not evidence), before and after: G1 p99 83.33
      and 83.33 ms, fps_avg 55.8 and 56.0, 6 and 6 resizes in the window, then p99 50.00 and 33.33, fps_avg 58.1
      and 58.5, 0 and 0 resizes; G9 p99 83.33 and 83.33, fps_avg 56.0 and 56.1, 6 and 6 resizes, then p99 33.33
      and 33.33, fps_avg 58.4 and 58.7, 0 and 0. Left, p99 still over 33.3 ms: 11-14 intervals of 2-3 refreshes
      per window, two per twist. The first follows the bind's frame-thread hole of 31-32 ms (`bind` 13-19 ms:
      `state_digest` 2.8-3.5, `feed` 1.8-6.4, `capmap` 2.5-6.7, `tint` 3.0-3.5 at a bound state, `flush` 2.8-6.1);
      the second follows a hole of 40-55 ms about 50 ms later that no `steps` row covers (the nearest is `shown`,
      `describe` 2.0-2.8 ms): the next step is to time that frame's parts.

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
- **As built (partial, 26 November 2026):**
  - Landed: the chrome model `CutSlice` in `Model/JumbleViews.cs`: the held split through `ViewPanes.Split`, the shell control, the preview that names the twist on release only, and the blocked cap's refusal. Its three console tests are in `tests/D02ViewTests.cs`.
  - Held key placeholders, both row requests: `Backslash` for the split and `Down` for the preview. `S` is seat 4 in the keymap and `G` is `view.global`.
  - Landed (27 November 2026): the core slice (ABI 10's `sa2_view_style` at 348 bytes, `SA2_STYLE_CHANNEL_SLICE`, `styled.hlsl`'s `sliceVs`), the host and `c600-render` rows, and the shell wiring (`D02/ShellSlice.cs`, the `slice` script). A failed slice no longer blocks the other styles: `ShellStyle` sends them and Local's slots go without the slice until its inside layer is sent.
  - The GPU timeout of 26 November (nvlddmkm 14 and 153, LiveKernelEvent 0x117, 23:15:55 UTC) is not explained. The lead was refuted on WARP with the product's Local (orthographic, zoom 4.507, spread 1.7, cap 0): the cap test draws 105,216 triangles and 0.87 targets of pixel work, the slice's default shell 321,547 and 2.65, the whole shell 4,575,204 and 37.1, the whole shell in perspective at d4 1.02 3,590,931 and 46.3. `check_core.py --slice` asserts these bounds through the newest frame's pipeline statistics (`sa2_test_frame_statistics`, a self-test export outside the ABI header).
  - `SA2_D3D12_DEBUG=1` (report file `SA2_D3D12_DEBUG_OUT`) turns on the debug layer and DRED for the core's own device; off by default. With it, the slice run passed with no debug-layer message and no device removal; then `slice`, `overlays` and `two-views` passed at 1280 x 720 (two reruns after foreground-only failures), each with no nvlddmkm or LiveKernelEvent entry. Findings: the shared note `d02-gpu-tdr.md`.
  - Owner sees: Global whole and Local sliced side by side (the slice is on by default in Local); hold `Backslash` (placeholder) and Local parts into two linked copies.

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
- **As built (27 November 2026):**
  - Core: `sa2_view_focus` takes `SA2_PROJECTION_STEREOGRAPHIC` (2), which needs a focus cell and 0 ≤ `focus_cos`. A pane takes the flag `SA2_PANE_ANTIPODE` (1). The struct sizes are unchanged.
    - The chart is `c600_display::chart::Stereographic` with the pole −p, drawn at 2·q.xyz/(1 + q.w). It is a branch of the precompiled geometry shader (`viewFlags` bit 5, the antipode bit 6), not a pipeline of its own.
    - The clip uses the cap test's anchor; the antipode's test is half-open.
    - The CPU check `raw view` covers the refusals, det(−I) = +1 and the two charts' Jacobian signs.
  - `check_core.py --raw` on WARP:
    - At h = 0 the two charts draw 6,096,000 triangles together, the whole puzzle.
    - At h = 121/125 and 0.5 the chart draws exactly the cap test's triangles (103,500 and 1,301,456).
    - The antipode chart draws within 2% of those counts (102,652 and 1,276,842), because the sticker triangulation is not symmetric under −I.
    - A turn about the pole (the camera's spin in the screen plane, 0.6 rad) lands 15,799 of 15,799 covered pixels, and 15,565 of 15,565 in the antipode chart.
  - Host and `c600-render`: `PaneSpec.flags`; `m6h_set_pane_flags`, appended, which a later `m6h_set_pane` resets to 0; the test `pane_flags_reach_the_core`.
  - Chrome model: `Model/RawView.cs`. It is a file of its own, not in `JumbleViews.cs`, because the colour step was editing in parallel. The three console tests are in `tests/D02RawTests.cs`.
  - Shell: `D02/ShellRaw.cs`, and in `Shell.cs` Local's focus, panes and key routing.
    - With the raw view on, Local's slot draws no slice (`ShellStyle`).
    - The dump has a `raw` row (`ShellTint`).
    - The palette rows and Local's border marks are wired in `ShellChrome` and `JumbleHost`: the caption always, and "antipode" on the antipode chart.
  - Keys, all placeholders and row requests: `Home` turns the raw view on or off; `End` adds or removes the antipode chart and turns the raw view on with it; `PageUp` and `PageDown` narrow and widen the clip by 0.02.
    - The palette rows are `view.mode.raw`, `view.mode.raw.antipode` and `view.mode.raw.clip <h | cut | tick>`. F1 › `view.mode.raw` is the row one step away.
  - The clip runs from 0 (90°) to the cut (121/125). It never goes inside the cut, so Local is never empty.
  - The static tick is h = φ²/4 = cos 49.12°: there all 80 meeting poles fall inside the clip. They are the first six shells of cell centres about a pole (4, 12, 24, 12, 4 and 24). Local's line reads the tick; there is no detent and no mark in the chart.
  - Runs at 1280 x 720 on wj-ia-300, build `c9-product-259e21c72c285d91`. The System log had no nvlddmkm or LiveKernelEvent entry after any run.
    - `raw` passed on its rerun, after a foreground-only failure, with 344 of 1,135 drawn composite frames having three panes.
    - Then `slice`, `two-views` and `pick` passed.
    - After text-only fixes for the source-pinning tests (the export order, `JumbleHost`'s delegate call and two `Shell.cs` lines), `raw` passed again on `c9-product-311037360a6485c8`, with 344 of 1,142 drawn composite frames having three panes.
  - Owner sees: the named cap's region flat beside Global after `Home` (or F1 › `view.mode.raw`); `End` adds the antipode chart next to it, labelled "antipode", with the caption on Local's border and line.
  - Left: holes drawn as faces (ADR-D02.25), and the tick as a mark on a drawn clip control.

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
- **As built, bare (drawing track, 26 November 2026):** the ring sheet as a second kind of B23's layout. It was added to ABI 10 before any release, so the host stays at version 13.
  - `SA2_LAYOUT_RING_SHEET` (2u) builds Look Lab's `GlobalLayout` sheet charts into the same Hopf table, from row 2611 on: SheetDx 0.68, SheetDy 0.57, SheetScale 0.6. Each cell's centre sits at (0.408 (h/2 - 14.75), -0.342 (r - 9.5)).
  - The chart is chosen as for the rings. The camera does not turn the sheet.
  - First held back, then fixed:
    - On the owner's GPU the sheet's first product frames removed the core's device (`DXGI_ERROR_DEVICE_HUNG`, 0x887a0006), and each recovery hung again. The shell held the sheet back (`ShellHopf.Opens`).
    - Cause, found after B17: `sa2_view_layout_set` built the Hopf table only for `SA2_LAYOUT_HOPF_RINGS`. A sheet named first therefore drew from an empty table at t18, a root SRV that nothing bounds. The chart index read from outside the table sent the next reads anywhere.
    - WARP tolerated the out-of-bounds reads, and the core's own checks always named the rings before the sheet.
    - Fixed in three places:
      - both Hopf kinds build the table;
      - `set_shape` refuses a Hopf layout before the table exists;
      - `hopfVertex` clamps the chart index to 599.
    - New checks:
      - `product_sheet_first` draws the sheet on a fresh context before any rings, and it matches the sheet drawn after them;
      - a CPU check refuses a Hopf layout without the table.
    - `ShellHopf.Opens` now lets the sheet through.
  - Not built yet:
    - the host's per-revision nearest-cell assignment (the shader's Hopf-coordinate choice stands in for it);
    - the wrap mark and the caption;
    - the no-sweep rule and the moved-set outline;
    - the palette's preset rows (the command table is left alone);
    - a click in a chart selecting the piece it shows.
  - Checks:
    - `check_core.py --hopf` passed. On the CPU, each cell's centre lies at its own place on the sheet, 600 distinct places. On WARP the sheet covered 50,820 pixels around the centre, differs from the rings and is not turned by the camera, and view 3's rings stayed as they were.
    - `tests/test_d02_drawing.py` passed 30 of 30, `test_renderer_core.py` 26 of 26 and `test_renderer_shell.py` 84 of 84.
    - `cargo test -p c600-render`, the bridge workspace and `test_render_ffi.py` passed.
    - With the sheet held back, `d02_run.py --script hopf --state wj-ia-300` passed again: 317 frames. Its first try failed only on the foreground rule, with a browser window in front; the retry passed.
    - With the sheet held back, `--script ring-sheet` drew 316 chain frames with no device removal and stayed red as intended.
    - After the fix, `check_core.py --style --hopf --panes` passed, the sheet-first picture included. `d02_run.py --script ring-sheet --state wj-ia-300` passed on the owner's GPU: 316 frames, all on device epoch 1, with no recovery and no warning.
- **As built (B22 rest, drawing track, 27 November 2026):**
  - Built:
    - The wrap mark and the caption. Global's border names the chart it draws (`JumbleViews.ChartMarks`): "ring sheet", the wrap mark "columns wrap" and the caption "only row neighbours are face neighbours" (sentence `ring_sheet.caption`). The Hopf rings show "Hopf rings". Built words `global.ring_sheet`, `global.hopf_rings` and `global.wrap` sit in `jumble-words.json`, and `test_d02_words.py` reads "columns" and "wrap" as plain words.
    - The palette's preset rows. `JumbleHost` carries the presets `view.preset.global.ring_sheet` and `view.preset.global.hopf_rings`. Two draft rows are appended to `command-rows-d02.json`, both naming CR-14: `view.preset.global.chart` (the palette's chart row, which applies the ring sheet unless the Hopf rings were picked) and `view.preset.global.chain` (back to the projection chain). The shell switches Global's layout at the next step (`ShellHopf.Request`, `Shell.cs`), and the border follows (`ShellHopf.Applied`). The command table is untouched.
    - The no-sweep rule. In the core, a view with a Hopf layout (viewFlags bit 3) draws no turn (`geometry.hlsl`): under a chart Global draws each revision's own poses at every phase, while Local's chain sweeps.
    - The moved-set outline. With a chart in Global, a committed sweep also outlines the moved set for `jumble.half_turn`'s fixed time (`ShellTurn`'s outline, B18 rest's flash path, B16's outline channel, palette entry 15). Local's panes zero entry 15's alpha while only that outline shows. With the slice in Local at a cap other than the grip cap, the twist is static in every view.
    - A click in a chart selects the piece it shows. This is B17's pick path unchanged: the core's ID pass draws with the view's layout.
  - Not built yet:
    - The host's per-revision nearest-cell assignment. The shader's Hopf-coordinate choice still stands in for it. It needs a new per-revision chart channel in the core (one chart index per slot, bound to the revision as the piece style is), which the host fills in Rust at each adoption, with ties going to the lowest cell id. The shader reads that channel under a Hopf layout. Its checks are the host test `the_chart_assignment_is_nearest_cell_lowest_id` and `check_core.py --chart`.
  - Checks:
    - `check_core.py --style --hopf --panes` passed on WARP. Its new check sets a transform at phase 0: the ring sheet and the Hopf rings draw exactly as without it, and a chain view turns.
    - `test_d02_drawing.py` has two new tests: the run's marks, clicks and chart rows, and the no-sweep wiring. `test_d02_words.py`, `test_renderer_core.py`, `test_renderer_shell.py`, `test_shared_chrome.py`, `test_render_ffi.py` and `tools/looklab/check.py` passed.
    - Product runs on wj-ia-300 at 1280x720 on the owner's GPU, with no nvlddmkm 14 or 153 event and no LiveKernelEvent after any of them:
      - `ring-sheet` passed: 567 frames. The border showed the three sheet marks, and both clicks selected the piece their sticker belongs to. The chain row showed the chain mark, and the chart row showed the sheet marks again. A 5 x 5 probe over the opening hit stickers only at those two points.
      - `hopf`, `pick`, `two-views` and `sweep` passed. The first tries of `hopf` and `two-views` failed only on the foreground rule; each passed when rerun alone.
      - `sweep` without a state also passed. Under the owner's Hopf-ring opening, J flashes. K is static in every view, because Local frames another cell, and Global outlined K's moved set once, for 336 ms. `expect_sweep` reads this chart case; on the chain K still sweeps.
    - The assignment's time per revision is not recorded, because the assignment is not built.
- **As built (item 1, drawing track, 27 November 2026):**
  - Built:
    - The host's chart assignment (`renderer/bridge/host/src/charts.rs`). The host reads the 600 cell centres once, as the core places them (each cell's frame times the manifest's `mesh.normal`, normalized). Per revision, on the worker thread that builds the adoption (`deferred::build`), each slot's chart is the cell nearest its own cell's centre under its piece's pose; cosines within 1e-6 are a tie, and the lowest cell id wins. One search runs per distinct cell and pose. An unposed revision gives each slot its own cell. Local never reads it.
    - The core's chart channel: `sa2_charts_set(ctx, revision, charts, count)`, appended to ABI 10 with no new number. It takes one cell id per slot for the adopted revision and refuses an id past 599, a short channel, another revision, NULL with a count, and no context. The core packs it into 16 bits per slot (the id plus 1; 0 means unset), keys the engine copy by it as it keys the piece style, and binds it as root SRV t20 in the product and styled roots. An adoption drops it. `geometry.hlsl`'s `hopfVertex` reads it under a Hopf layout and falls back to the Hopf-coordinate choice for an unset slot. A chain view never reads it.
    - The wiring: the bridge's `CoreFunctions` has 28 symbols with `charts_set` last, `c600-render`'s `Backend::set_charts` is best effort, and the shell's `CoreLibrary.cs` loads the new export. At each install, the host sends the revision's channel after the digests.
  - Checks:
    - The host test `the_chart_assignment_is_nearest_cell_lowest_id` covers the identity, each centre being its own nearest cell, ties to the lower id both ways round (including a tie with cell 0), and three binary32 poses against a per-slot reference.
    - `check_core.py --style --colours --hopf --chart --panes` passed on WARP. The new chart check adopts a 12° turn along J's fibres. Six sample cells draw on the ring sheet inside the rectangle of the cell the turn takes them to (pitch measured from the next chart along the ring, 16.5 to 17 px at 640x360, 2 px of slack). Every sample moves out under a wrong channel (each slot's own cell). The Hopf-coordinate choice draws the same picture, a chain view ignores the channel, the refusals hold, and an adoption drops the channel.
    - `cargo test -p c600-render-ffi -p c600-shell-host` (renderer/bridge) and `cargo test -p c600-render` (rust) passed.
    - `test_d02_drawing.py` (the shader's chart line, the 28th symbol), `test_renderer_core.py` (the ABI 10 declaration), `test_d02_words.py`, `test_renderer_shell.py`, `test_shared_chrome.py`, `test_render_ffi.py`, `build_product.py` and `tools/looklab/check.py` passed.
    - Product runs on wj-ia-300 at 1280x720 on the owner's GPU, `ring-sheet` first and alone, each passing at its first try with no nvlddmkm 14 or 153 event and no LiveKernelEvent after it: `ring-sheet` (561 frames; the three sheet marks; of its two clicks one selected a piece, where the stand-in's picture had both on stickers), `hopf` (298 frames), `scene-look`, `two-views` (1,279 frames) and `pick` (11 clicks, 2 hits, 0 stale).
  - Owner sees: `watch_d02.py --state wj-ia-300`, palette row `view.preset.global.chart`: Global's ring sheet draws each piece in the rectangle of the cell nearest its posed place.
  - Machine (the assignment's time per revision, release build on the integrator's machine): 0.3 ms unposed, 7.3 ms with three poses, and 276 ms in the worst case, a generic pose for every piece, where each slot is its own search. It runs off the frame thread.

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
- **As built, bare (drawing track, 26 November 2026):** the Hopf-ring opening, pulled ahead of B22 as noted under B6a.
  - The core gains ABI 10, `sa2_view_layout_set`, and the host version 13, `m6h_set_view_layout`. The 1560-byte row carries Look Lab's `fixtures/hopf-rings.json`: J, K, the 20 ring origins, and each cell's ring and half-step.
  - The core checks the rings against its own cells, then builds Look Lab's `GlobalLayout` ring charts into one table that the product passes read at t18.
    - Each cell centre must project onto its ring's plane at the phase of its half-step, within 0.01 of a half-step.
    - The projected length must exceed 0.99. Ring cells lie at 0.99555, not 1: they wind as a helix around the great circle, not on it.
  - With `--d02`, when the opening file names `hopfRings`, Global opens on the rings. Its zoom is the file's 1/tan(fov/2), and the linked 3D spin turns it.
  - Departure: the chart is chosen in the shader from the posed centre of the sticker's own cell, in Hopf coordinates: the nearest of the 20 base points, then the nearest phase along that ring. The host's per-revision nearest-cell assignment stays B22's.
  - Not built yet:
    - the preset row and the palette switch;
    - the no-sweep rule;
    - `--chart hopf` as a separate flag (the check is `--hopf`).
  - Checks:
    - `check_core.py --hopf` passed on CPU and WARP. On the CPU, the table puts the 20 ring centres on a dodecahedron's vertices. The chain layout and NULL draw byte-identically. Seven malformed or unfitting rows are refused. The camera turns the rings. Another view keeps the chain, and a third view with the same rings draws the same picture. The WARP picture covered 9,874 pixels, centred.
    - `d02_run.py --script hopf --state wj-ia-300` passed: opening zoom 2.2095 = 1/tan(0.425), 2.6735 after two notches, 316 one-view frames.
    - `tests/test_d02_drawing.py` passed 28 of 28, `test_renderer_shell.py` 84 of 84, `test_renderer_core.py` 26 of 26 and `test_renderer_acceptance.py` 54 of 54.
    - `cargo test -p c600-render`, the bridge workspace and `test_render_ffi.py` passed, including `the_host_sets_a_view_layout`.
  - Not checked: the product window's picture of posed pieces on the rings, and a hardware run of `--hopf`.

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
- **As built, gate tools (B24, 27 November 2026): tools built, sittings not run.**
  - Rows G4 to G11 are `gate_d02.py --row <id>` with the scripts `d02_scripts/gate-g4.json` and onwards.
  - `gate_d02.py --occlusion` measures occlusion: both chart presets at the default camera, a grid of picks over Global
    (one pick pass per point), and the distinct selected pieces per preset. It is a sampled count, not a full ID-pass
    count.
  - The prerequisites (twist sweeps on `deep-2`, the foreground question) are those of B19. Results go on
    `docs/progress/1.0/adr-d02-gate-results.md`.
  - Fixed on 27 November 2026 (gate track): G4, G6 and G8 select their grip cap as B19's rows do. The occlusion grid
    lies over Global's opening as the shell lays it out (a click step's fractions are of that opening; the default
    region is all of it). The record holds the opening the dumps report (`frame.global_opening`) and checks that every
    click landed in it. Not yet run.
  - LF1 (27 November 2026): G4 and G6 set the Local mode their rows name, after naming the cap and before the repeat
    block: G4 `jumble.slice.shell default` (Local's slice), G6 `jumble.slice.shell 3.14` (the shell at its limit, so
    Local draws its whole chain and carries the sweeps). `ROWS` holds the argument as `local`;
    `tests/test_gate_row.py` checks both scripts, also after the sweep cap moves. No dry run.
  - LF2 (27 November 2026): G7, which also names "Local chain", sets `jumble.slice.shell 3.14` after naming the cap
    and before the repeat block, with `"local": "3.14"` in `ROWS` and the script's note; LF1's test covers G7 too.
    No dry run.

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
- **As built (26 November 2026):**
  - The seed 7001 word is the sample `seed7001-words.json`, which `make_d02_samples.py` puts from the tutorial runner's word files. Its nine words are one twist each, so the restoration is nine twists long.
  - `Model/JumbleWords.cs`: `WordTree` (folded in word order, parts checked to chain), `WordPlayback` (by part, twist by twist in an opened part, numerals only above the bound), `LongWordBound` (10,000, labelled as X3's placeholder), the crossing sentence, `WordSignature` and `LogTabMark`.
  - A lattice-to-lattice word's 35 slots are pending until the build ships their data; its ν row shows. A word that leaves the lattice shows the undefined numeral in all 35.
  - Departure: the word plays in a Word panel in the left dock (`Ui/WordPanel.cs`), not in `DraftLine.cs` and `LibraryPanel.cs`. The product shell has no Library panel of its own yet, and K2 wires the real commit bar. The panel shows the commit bar's line (length and alphabet), the folded tree, the counter, the state after each part (N and digest from the samples), the signature, the log tab's mark and the crossing sentence with its reachability.
  - Checks: the four console tests; `d02_run.py --script seed7001` PASS (each part's digest is its word file's output, N from the sample totals, solved at the end).

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
- **As built (26 November 2026):**
  - `Model/NuBand.cs`: `NuConstants` (read from the catalogue's `nu_constants`: 45 names in row order, moduli 2 and 5, the 8 frame groups, the 2 dials, the 9 always-zero coordinates), `NuBand` (one cell per rail cell in the rail's order; diamond marks, outside the ring family; O17 and O28 as five-position dials; fixed dots; undefined off the lattice; not shown on the lattice above N = 0), `NuCorrections` (36 rows in id order, selection, signed marks, preview, check) and `NuCorrectionCheck` ("ν after this set", "labels not predicted").
  - `Ui/NuBand.cs`: `NuBandStrip`, which `OrbitRail.AddBand` puts in the rail's own column under its cells, so the 35 columns align and follow the order setting; `NuCorrectionsPanel` in the left dock, headed "Library › Built-in › ν-corrections": Up, Down, Home and End move; Space marks +1 and Shift+Space -1; Delete unmarks; Shift+Enter previews; Ctrl+Enter checks the set. The host carries `jumble.corrections.mark` (CR-14) and `jumble.corrections.check` from the palette too.
  - Departure: the rows sit in their own dock panel, not in `LibraryPanel.cs`, as B25's Word panel does; the product shell's Library holds the lab's sample records until K2. Shift+Enter previews the row's ν effect in the panel; staging its letters as a commit-bar preview waits for K2's backend.
  - `build_d02_fixtures.py` now builds the correction states as on-zero plus their catalogue letters. nu-c0 (9 letters) is written; nu-c35 is 5,629 letters; its write ran over ten minutes without finishing and was stopped (risk R2), so it is not written yet. The `nu-band` script needs only nu-c0.
  - Checks: the five console tests; `d02_run.py --script nu-band --state nu-c0` PASS (the band under the rail, 35 cells, b8 and b25 marked, both dials at 0, 9 fixed dots; c00 and c35 marked, c35 previewed, the set checked: ν after this set is b25, nu-c35's ν). One earlier run failed only its foreground check, when another window took the foreground (`owner_acted`); the rerun passed.
  - Long word fixtures (26 November 2026): the time is not in Python. `build_d02_fixtures.py` runs the host's Rust writer, and a pose-move session costs about the same to write, to check at the close and to reopen: the command layer derives each pose move from the head and, every 64 moves (`POSE_CHAIN` in `c600-command`), replays the whole jumbling journal flat from solved, so each pass grows with the square of the word's length (nu-c35's first 1,024 letters: commits 39.1 s, control 44.2 s, timed reopen 41.1 s). The writer now times its reopen on a thread beside the control (`fixture::write_word_with`; the copy gets an empty `lock` file, since the live layer may still hold the session's), not after it; the records are byte-identical. Measured: the first 1,024 letters, each run alone, 124.7 s before and 74.7 s after; nu-c35 with both writers running side by side, 3,565.9 s before and 2,749.2 s after; nu-c35 with the new writer alone, 2,034.6 s (commits about 767 s). nu-c35 is written and matches the sample's three digests (nu-c0 and wj-ia-300 were rebuilt through the script first and match too). Its `reopen_ms`, 1,243,834, is timed beside the control and overstates a launch: the old writer's timed reopen of the same session, run alone, took 765,022 ms, which a product launch at nu-c35 pays each time (risk R2). Making long pose-move sessions fast to write and to reopen needs the command layer to keep a flat reachability claim without replaying from solved; that is a layer change, not a fixture script change.

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
- **As built (26 November 2026):**
  - `keymap.json` gains a `seats` section, which only the default file may carry: the pinned menu (I_a and its identity), the fifth-turn g = I_a[12], the 11 retained twist keys (items 1 to 11) and the four held seats. Seat k's key alone types g^k; held with retained key r it types r·g^k, g^k performed first (matrices act on column vectors). Each seat carries 3 twists of order 2, 3 of order 3 and 6 of order 5, and the four seats carry the 48 jumble twists. The placeholder keys avoid every bound key, the sample's host keys, the live session's J K L and the digits: seats Z, X, C and S (glyphs S1 to S4); retained I ; ' , . / [ ] - = `.
  - `Model/KeyMap.cs`: `SeatKeys` reads and checks the section, and `KeyMapModel.HeldSeat(seats, seat)` gives the 12 twist keys with their chords and orders. `Caps` takes the seats: with one seat key held and no modifier, the seat key shows held and its 11 retained keys show as twist keys, each labelled with its order. At rest each seat key shows its seat and glyph. `KeyMapPanel.Seats` is set by the product shell under `--d02` with `--sample`. That is the only place the key map is mounted.
  - `Model/JumbleViews.cs` is new. While Jumble is on, Global's border gains a link mark, "projection chain (600-cell-Full)". Both borders gain a not-drawn mark, "exposed faces unavailable", in the role `state.exposed_face.unavailable`, with `input.exposed_geometry_unavailable` in its tooltip. The same words end the named cap's Tier 1 in the cap map's panel and the inspected cap's Tier 1 in Cap covers. `jumble-words.json` gains a `built` list for words the build composes from an item rather than quotes; it holds the chain's words.
  - `tests/test_d02_words.py` reads, longest phrase first, the sentences, names, synonyms and row names, the built words, the draft rows' labels and the seat labels. The rows' notes are help prose that K2 replaces, so they are not read. A word must be one of the following:
    - a glossary row, found through `build.py`'s own index;
    - a `PENDING_TERMS` entry, which must match section 15;
    - a reading of one row, such as F_c → Floor or rev → Revision;
    - a homonym's sense in D02 (twist, projection, covers), a name or a variable;
    - a plain word: ordinary English or general geometry such as face, ring and antipode, which is never a glossary name.

    An unlisted word fails. A pending term that gains a glossary row must leave the list.
  - Departures:
    - The files list grows by `Model/Commands.cs` (the optional section), `Model/CapCovers.cs` and `Ui/JumbleSurface.cs` (Tier 1), `Ui/JumbleHost.cs` (the marks), `renderer/shell/project/ShellChrome.cs` (the seats) and `jumble-words.json`.
    - No chord routes yet. The draft row `jumble.seat.hold` waits for CR-19, and the keys and glyphs wait for XM1 and G6.
  - Checks:
    - the four console tests and the other 223 (227 PASS), and `tools/looklab/check.py` PASS with the checkout unchanged;
    - `python tests/test_d02_words.py` OK;
    - `glossary/build.py --check` unchanged (298 terms);
    - `build_product.py` PASS;
    - `test_renderer_shell.py`, `test_shared_chrome.py`, `test_d02_drawing.py`, `test_renderer_core.py` and `test_d02_samples.py` OK.

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
- **As built (26 November 2026):**
  - Part 1 (b6e12f9b): `Model/Research.cs` holds `IResearchSource` (one method per research row and the draft `RecordText`), `SampleResearchSource` (synthetic; it answers the reading rows, and the rows that run, import or export refuse until K2), `ResearchWords` (ADR-D02.46's words; `ColourRole` is null for every kind), `ResearchRows` and `ResearchModel` (tabs, the in-place record with its folded tree of named parts, names, copy and save with `Origin.Research`, current and stale answers, the pinned input). `Ui/ResearchPane.cs` is the Library panel's port: tabs, a head with the counts and `evidence: synthetic`, a carried line, and a line-stepped virtual list whose every control sends its row. `data/samples/research/panel.json` holds 8 records (stages 15, 16 and 18), 8 claim groups and 5 runs, with field names from `rust/c600-research/src/panel/answers.rs` and `wire.rs`.
  - `renderer/shell/project/D02/ShellResearch.cs` mounts the pane in every mode, so Analyse never shows a missing pane; without the sample it says "Research rows wait for K2". In sample mode it reads `res://samples/research/panel.json` (or `M600_D02_DATA`), registers the sample's draft row `research.record.read`, which the palette marks as a draft row, and opens the first failing claim group. Its `Dump` writes the tab, the open object, the counts, staleness, the carried line, the names and the lines for a run.
  - `ShellChrome.cs` routes the pane's rows: a research row naming a research object, or a tab row, is available and runs first, before `workspace.set` (a name is `workspace.set research.names:<ref>=<name>`), and its words go to the status line. The pane's controls send their rows through the palette's path, and a commit, undo or redo marks the answers stale.
  - `workspaces.json`: Analyse's centre splits horizontally (0.58): the pair and the Preview and Canvas tabs above, the Research tab below.
  - Departures:
    - The files list grows by `ShellChrome.cs`, `ShellJumble.cs` (the dump), `renderer/shell/d02_run.py`, the `research-pane` script and `tests/test_renderer_shell.py`.
    - Look Lab Live shows "Pane not available" for the Research tab in Analyse, because Live is not edited; the pane is mounted in the product only.
    - The sample opens the first failing claim group at load, so the owner sees it one workspace tab away; the script folds it and opens it again with `object.inspect`.
    - `Ui/ResearchPane.cs` changed after the first product run hung when Analyse first showed the pane: the list sat in a container, so each rebuild of fresh, zero-width wrapped labels resized it and its resize rebuilt it again. The list now hangs in a plain clipping window whose resize lays the lines.
    - The Runs and Obligations tabs show their counts only; their rows come in B29.
    - The revision's nine keys are checked against the command layer's `Revision` (`c600-command/src/receipt.rs`), which `wire.rs` writes.
  - Row requests: `research.record.read` (a query that reads a record's finding and named parts); the researcher's name as a `workspace.set` write of the record `research.names` (payload form open); a command-icons entry or a label-only mark for `research.record.read`.
  - Checks:
    - the six console tests and the other 248 (254 PASS), and `tools/looklab/check.py` PASS with the checkout unchanged;
    - `python tests/test_d02_research_sample.py` OK (7 tests: format and synthetic evidence, the envelope and one research digest, the panel's field names, the draft row outside the command table, every verdict kind, part sums, claim groups' records in the inventory);
    - `build_product.py` PASS;
    - `test_renderer_shell.py`, `test_shared_chrome.py` and `test_d02_drawing.py` OK;
    - `d02_run.py --script research-pane --state wj-ia-300` PASS at 1280x720 (7 steps: Solve shows no research pane; `workspace.set layout:analyse` shows it with the Fail row open on "fails within on one instance", "first claim that does not hold: cycle.closes" and "code: KN6:evidence.certificate_rejected"; `object.inspect` folds the row and opens it again; the head says `evidence: synthetic`; the error lamp counts no failure).

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
- **As built (26 November 2026):**
  - `Model/Research.cs`: `RunBudget` (steps); `SeedPool` (ADR-D02.52: a seed's words name its scored block, J2 7401 to 7406 or J3 7407 to 7421 with the block's depth, or free exploration from 7422 to 7499); `RunDeclaration`; `ResearchRun` gains the code, the covered steps, the checks' verdict kinds, and the seed, depth and budget a started run recorded, and its `State` is ADR-D01.61's `SlotState.BudgetStopped` (ported from `ToolSlots.cs`) when the reading is `cut_off` or the covered steps reach the budget; `ResearchObligation`. The model declares a field (`research.run.declare seed=|depth=|budget=`), starts the declared run (`research.run.start`, unavailable with "declare a budget before the run starts (ADR-D02.4)" until the budget, then the seed and the depth, are declared; the answer must record what was declared), reproduces a run (`research.run.reproduce`, DET.7's answer in ADR-D02.46's words) and says why a research row is unavailable (the two CR-17 rows with "row not in the command table yet (CR-17)"; the free view also "refused past the four-view envelope: four views are open" at `ViewPanes.MaxPanes`). Runs lists the declaration, this session's runs, then `research.runs`' runs by stage and family; an open run shows its seed's words, its budget "declared before the start and recorded", the cut-off line and code, its checks, the pinned input, its reproduction and the CR-17 line with the caption "derived; reachability open". Obligations lists the record-only anchors by stage and family, each with what it misses word for word, its label, statement and exports.
  - `Ui/ResearchPane.cs`: the declaration line (seed, depth and budget fields, each sending `research.run.declare`, and Start); a control with a reason is drawn disabled with the reason as its tooltip, and the reason is also a line of the open run; actions wrap in a flow container.
  - The sample adds answers of `research.run.start` (a draft row: `research.runs`' run form plus the recorded `seed`, `depth` and `budget`) and `research.run.reproduce` (DET.7's `same`), and the draft rows `research.run.declare` and `research.run.start`. Its one run never finishes: the source writes the declared seed, depth and budget into it, covered equal to the budget, and names it by the SHA-256 of its declaration (`SampleResearchSource.RunId`), so a script names it before it starts. It reproduces only a run it started.
  - Shell: `ShellChrome.cs` asks the research model why a research row is unavailable, and a research row run from another layout switches to Analyse, so Runs is one palette row (`research.runs`) from the opening Solve layout. `ShellResearch.Dump` adds the declaration, the start's state, the started runs, their reproductions, History's entry count, the open run's CR-17 reasons and the frame's notice.
  - Departures:
    - The files list grows by `ShellChrome.cs`, `ShellResearch.cs`, `renderer/shell/d02_run.py` (`expect_research`), the `research` script, `tests/test_d02_research_sample.py`, `tests/test_renderer_shell.py` and `tools/looklab/tests/Program.cs`.
    - The budget is in steps, the unit of the cut-off's code `KN7:budget.steps`; ADR-D02.53's runner hours are not a field.
    - "differs at byte k": LP5's mismatch `LG4:evidence.reproducibility_mismatch` names no byte, so the pane words one only when the answer carries a `byte` (a row request; tested in the console).
    - The shell's open-view count stays at its default of one; the envelope refusal is tested in the console, and the shell's count comes with the free-view row (CR-17).
    - The sample run's branch (its pinned input) is the listed cut-off run's.
  - Row requests: `research.run.declare` and `research.run.start` (a research run on the seed pool within a declared budget; the answer is `research.runs`' run form with the recorded seed, depth and budget); `research.run.reproduce`'s mismatch naming its first differing byte; CR-17's two rows (`research.run.view`, `research.check.trace`, already draft rows); command-icons entries or label-only marks for the two new draft rows.
  - Checks:
    - the five console tests and the other 262 (267 PASS), and `tools/looklab/check.py` PASS with the checkout unchanged;
    - `python tests/test_d02_research_sample.py` OK (8 tests; new: the sample run is cut off at its budget with KN7's code and reproduces as `same`);
    - `build_product.py` PASS; `test_renderer_shell.py`, `test_shared_chrome.py`, `test_d02_drawing.py` OK; `tools/wiki/lint.py` ok;
    - `d02_run.py --script research --state wj-ia-300` PASS at 1280x720 (18 steps: `research.runs` from Solve brings Analyse on Runs; a start before any declaration is refused with ADR-D02.4's reason; budget 4096, seed 7430 (free exploration) and depth 40 declared; the run starts, is `BudgetStopped`, reads "not found within budget" with "the run stopped at its budget: no verdict (covered 4096)"; Reproduce reads "reproduced: bytes equal"; both CR-17 rows are refused with their reason; Obligations shows four anchors; `object.inspect` opens the run again; History's entry count never changes and the error lamp counts no failure); `--script research-pane` still PASS.
  - Owner sees: in the palette, `research.runs` (Research: Runs) opens Analyse on the Runs tab; type a budget, a seed and a depth, Start, and the run reads "not found within budget"; Reproduce reads "reproduced: bytes equal"; the two CR-17 buttons are greyed with their reason.

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
- **As built (B30 agent, 26 November 2026):**
  - `Model/Tuning.cs`:
    - `TuningValue`: a family, its scene look and the five seed knobs (ADR-D01.50), written into an `Appearance` and nothing else.
    - `Tuning`: `Open(frame)` shows the panel as a tab of the current layout's right dock. `SetFamily`, `SetLook`, `SetKnob` (clamped to the knob's range) and `Tune`, which reads `theme.tune`'s `knob=value[,...]`.
    - `Strip(knob)`: the knob's live ramp strip of twelve steps. Tone, contrast and black level show the neutral ramp, and accent hue the accent ramp. Grid intensity shows the grid line's mix across its range, with its current step marked.
    - The compare: slots A and B, and the shown slot is the live tuning. `Swap` shows the other slot and keeps the live tuning in the slot it leaves, so two swaps are no change. `Hold` draws the other slot. `Release` restores the appearance exactly, through `Appearance.Store` and `Restore`. Every edit is refused while a compare is held.
    - The model holds only the appearance and its slots: no session, no puzzle state.
  - `Ui/TuningPanel.cs`:
    - Family and Look choices.
    - Each knob is a slider with its value and its ramp strip, drawn as the token specimen draws its swatches.
    - Set A, Set B, Swap, and Hold. Hold is held with the pointer, or with Space or Enter while it has the focus. The compare line and a status line for refusals.
    - Every edit goes through `AppearancePanel.Edit`, so the frame recolours in place. It is rebuilt only when a swap or a hold crosses families.
    - The knob slider, its value and the titles are shared with the Appearance panel through `AppearancePanel.KnobSlider`, `KnobValue`, `KnobText` and `Title`.
  - The shell:
    - `ShellChrome` registers the panel in the right dock while Jumble is on, with the glyph `panel.tuning` (`sliders-vertical`). Its rows are `tuning.open`, `tuning.slot.set`, `tuning.slot.swap`, `tuning.compare.hold` and the command table's `theme.tune`.
    - `ShellTint` redraws on the frame's `Recoloured`, so a knob change sends the core the new tint palette through `m6h_set_view_style`. The tint dump gains `palette_sha256`.
    - The `ShellJumble` dump gains `tuning`: the right dock, shown and folded, the slots, the hold, the frame's window colour, and the head's state digest.
  - Draft row `tuning.open` (CR-16) in `command-rows-d02.json`.
  - The panel reads no sample data; the runs use `wj-ia-300`'s session fixture.
  - Below 1600 px the right dock's panel folds to its rail (owner, 26 November 2026). So at 1280x720 the row opens the panel in the model and on the rail, and the knobs are one `theme.tune` row away. `d02_run.py --script tuning` passes there, with the click on the knob finding none. `tuning-wide` runs the same steps at 1920x1080 with the panel shown, including the click on the tone knob. Both runs keep the state digest and the tint bytes after every step.

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
- **As built (B31 agent, 26 November 2026):**
  - B30's gap: below 1600 px the right dock folds to its rail, so `tuning.open` opened a panel nobody saw. `FrameModel.RevealDock` and `PressRail` now open a folded dock the way a click on its rail does: the panel is drawn over the fold until the rail is pressed again, and the fold rule (`ChromeFrame.Folds`) is unchanged. `d02_run.py --script tuning` passes at 1280x720 with the tone knob found and clicked.
  - `Model/Tuning.cs`:
    - `TuningValue` gains the Advanced role overrides (OkLCH, per family), stored in `Appearance` beside the knobs and passed to `TokenSet` as overrides.
    - `TuningPreset` reads the new `tools/looklab/data/design/tuning-presets.json` (Graphite and Paper, each with an Inspecting variant), checking each preset against a scratch appearance. Shipped presets are read-only: the first edit forks one into `<id>.copy` ("<Name> copy"), and Save fills the fork. Presets live in memory; nothing is written to disk.
    - `Save()` runs `SaveIssues(TokenSet)` first and refuses in place with the first failing pair: the token set's contrast rules; the height ramp (`tint.jumble.height.2/.4/.6/.above`) not strictly monotone in OKLab L; or two contact classes (1 to 6 and the overflow) closer than G4's bars (`StickerPalette.Needed`, `NeededSimulated`) under normal vision, protan, deutan or tritan.
    - Advanced: `AdvancedRoles(query)` searches the family's roles; `Override("role=L C h")` and `"role=reset"` set and clear an override.
    - The held simulation: `HoldSimulation`, `ReleaseSimulation` and `Matrix`, one row-major 3×3 matrix in linear RGB from `ColourMetric.SimulationMatrix` (Machado protan, deutan and tritan; `contrast` is the luminance matrix).
  - `Ui/TuningPanel.cs`: a Presets section (choice, Save, the preset line and the refusal list), a Simulation section (choice and a hold button), and Advanced behind a switch with a search field and up to twelve role rows (swatch, L C h field, Reset). A held simulation draws `SimulationFilter`, a screen-texture shader over the frame that applies the matrix and keeps alpha, and raises `Simulated`.
  - The core half: `ShellStyle.SetSimulation` puts the same matrix into the view style's matrix field, but only with `M600_D02_CORE_SIMULATION=1`. It is off by default, following the look-cost note (the core half waits for G8), so a held simulation changes the chrome and the panel says the pieces wait for G8.
  - Rows: the command table's `token.override.set` and `token.override.reset` (ADR-D01.50, .51), and CR-16's drafts `tuning.preset.apply`, `tuning.preset.save` and `tuning.simulate.hold`. No new row.
  - Both themes failed G4's simulated bar as shipped (graphite deutan classes 2 and 5 at 0.017). `tint.jumble.contact_class.1` moves from blue.8 to blue.7 and `.2` from teal.8 to teal.9 in `graphite.json` and `paper.json`; both now pass `Save()` (lowest ratio to the bar 1.23 and 1.15).
  - Tests: `D02BothThemesPassSaveAsShipped`, `D02ShippedPresetForksOnEdit`, `D02SaveFailsOnNonMonotoneHeightRamp`, `D02SaveFailsOnCollidingClassesUnderSimulation`, `D02SaveFailsOnContrastRule` and `D02SimulationAndRevealedDock`.
  - Run: `d02_run.py --script tuning-save --state wj-ia-300` at 1280x720. It reveals the panel over the fold, applies Graphite, forks it with a knob and saves, holds deutan over the chrome (the core gets the identity), darkens `tint.jumble.height.6` through Advanced, and sees Save refused with the pair `tint.jumble.height.4` and `.6`. It then resets, saves and gets the shipped preset back. The state digest and the tint's outline and fill bytes never move.
  - Owner sees, in `watch_d02.py --state wj-ia-300`: the palette row `tuning.open`, then the hold button under Simulation; the chrome changes, and the pieces do not until G8 turns the core half on. Advanced, `tint.jumble.height.6` set to `0.2 0 0`, then Save, is refused with its pair.

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
- **As built (B32 findings):** J1's gaps, closed at the narrowest layer (reused: B12's `RepairListControl` keys, B13's
  stage, B20's `CutSlice`, B6's `EntrySlot`, d02_run's player).
  - C5-8, the repair list's key route: `jumble.repair.focus` (CR-14's draft row) opens Local's Repair list and gives it
    the keys with no row focused (ADR-D02.32); then the list's own keys work: arrows focus, 1 to 9 or Enter pick, Left
    and Right walk the steps. With `k` it focuses row k, and with `k s` it shows step s (ADR-D02.34). ADR-D01.19's
    palette route, no new key. `jumble.repair.stage [k]` (CR-15's draft row) stages the row the solver names, or the
    focused one, as the solver's pick by the palette's Enter (ADR-D02.33); with neither it refuses. Both refuse without
    a named cap, without a list, or on a list that shows no rows. `RepairListModel.Focus` records its focus act.
  - The other draft rows: `view.preset.jumble_entry` applies the entry slot again (Jumble on only; a key changed since
    is set again); `jumble.slice.split` latches the held split on or off (Backslash's release also ends it);
    `jumble.slice.shell r` sets the shell's outer radius, 0.05 to π, or `default` (`CutSlice.SetShell`). No draft row of
    the five waits for a K2 server.
  - `watch_d02.py --script NAME` replays a d02_scripts/ script the way `d02_run.py --script` plays it (its puzzle, the
    shell's player, sample mode only for d02_run's sample scripts, the fixture its script needs); the window opens
    behind and quits after the last step. It refuses `--follow`.
  - `D02Window.Watch`: past the first six seconds the shell reorders no window but keeps noting the owner's acts on its
    own window (a move, a resize, the window in front) for the window's life. A mouse button elsewhere counts only in
    the first seconds, where it stops the hand-back.
  - F1 and Local's hint: in sample mode the key map holds F1 (ADR-D01.74), so the hint there names the palette's key,
    "Space › jumble.cap.name <id>"; outside sample mode F1 with no help topic opens the palette and the hint keeps
    ADR-D02.12's words. The open F1 question is not decided. The dump says `sample`; `d02_run.local_problems` reads the
    words of the mode. The cap map's refusal "F1 › jumble.cap.unname" (its tooltip, the cap prompt's rows and the
    status line) is routed the same way in sample mode (`JumbleHost.PaletteWords`, set by the shell to
    `ShellJumble.PaletteRoute`); outside sample mode it keeps F1, so `cap-hover`'s expectation stands.
  - ADR-D01.52 and .49/.5 (as built, 27 November 2026): the colour step's open lead, that the sticker palette and the
    scene look "are not resent after a host session reopen", is settled by a test: they are kept, and nothing resends
    them. Every new head the shell adopts (a twist, a fixture load, `jumble.on`/`off`, a state switch) reaches the core
    as `sa2_engine_adopt`, which drops only the revision-bound piece style, transform and chart channel; the table and
    the look are scene state, and the deferred worker's new host context (`Built.ctx`) holds neither. The WARP check
    `check_core.py --colours` now adopts a second revision with the same labels after setting both and finds the
    revision's frame byte-identical to the one before ("scene colours: kept across an adoption"); a lost table or look
    would draw the core's hues or the default background there.
  - J1: the five rows leave `not_run`; the session names cap 150 again, types `jumble.repair.focus`, Down and
    `jumble.repair.stage`, and latches the split off again. `repair-boundary` stays under `not_run`: J1 reads no list
    screens; the new `repair-keys` script checks the boundary by keys.
    Rule 1's check (`state_unchanged`) now counts a staged record against J1 only when no `jumble.repair.stage` row
    staged it: the stage row is the solver's pick, not a filter, view, tint or tool.
  - Checks: `check.py` (326 of 326), `test_shared_chrome.py` (15 OK), `build_product.py` (PASS),
    `test_renderer_shell.py` (106 OK), `test_d02_drawing.py` (63 OK), `test_d02_words.py` (6 OK),
    `test_d02_samples.py` (14 OK), `tools/wiki/lint.py` (ok); product runs `slice`, `two-views` and `repair-keys` on
    `wj-ia-300` PASS with no nvlddmkm or LiveKernelEvent entry; J1 (with `wj-ia-end`'s fixture written by the Rust
    writer) prints `M600_D02_J1_PASS operations=51 reached=51 boundary=0 observation=9/9`, no GPU event.

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
- **As built (C6 findings, 27 November 2026):**
  - No fault switch was built: `Model/Jumble.cs` has no `M600_D02_FAULT`, which Files above and section 7 name. J0
    plants each defect as data in its own copy of Look Lab's samples (`plant()` in `track_j.py`, read through
    `launch.py --d02-data`), so the product's code is unchanged. Six of section 7's classes cannot be planted that way (`NOT_PLANTABLE` gives each
    reason), and the result line says so: `not_planted=6`.
  - A defect may name a rule outside the driver's seventeen. It counts as planted, and only the researcher subagent
    can catch it, so the planted set can show the driver's blind spots. The same change under two ids, and an id whose
    prefix is not its writer (`by`), are refused.
  - The subagent catches a faulted run only when it says broken and names the defect's rule (for a rule outside the
    driver's, in its own words, which the record keeps for the integrator). Its false alarms on the clean runs fail J0
    like the driver's; an answer whose `broken` is not true or false is refused.
  - The packet's draw is seeded by the defects' sha256 and a nonce that only the sealed key keeps. The leak test also
    matches the out folder's path JSON-escaped and in any case.
  - Scoring takes Opus's count of the subagent's effort (`--effort`: scripts, dumps, minutes, words) and checks
    section 7's cap; each scoring keeps the answers' sha256, and earlier scorings stay in the record's history. The
    subagent's reach half has no driver function yet, and the line prints `reach=not_run`.
  - A check that throws is a run problem, not a catch. J1 counts the rules it observed, not the session's entries
    (`observation=6/6`, was 9/9), names exposed faces under `not_run`, and treats a screen with no BoundaryLedger
    count as a problem.
  - LF2 (27 November 2026): the driver's eighteenth rule, `total-n`: Global's total N is the state's own seam count
    (its totals' `N`), never the sum of the caps' N_c. The J0 session reads Global's border N at `start` and after
    `jumble.cap.name 150`; at wj-ia-300 the two differ (98048 against 26925). `tests/test_renderer_shell.py` flags a
    faulted screen and passes a clean one. Not run over J0's clean runs: the driver has no no-defects mode
    (`defects.json` is always read).

### ADR-D01 ports (as built)

**As built (ADR-D01.75 and ADR-D01.13's ghosting; 27 November 2026, branch `claude/d02-research`, gpt-6.2-sol call
20261127T063126Z-e21b0212).** The grip's selection, its Grip frame marks and the modifier ghost, on B17's picks and B18's key route.
- Host (version 14, no new number): `m6h_grip_report` and `m6h_grip_set(cap, frame)`, answered through `m6h_answer`
  (`renderer/bridge/host/src/grip.rs`, `turns.rs`). The twelve proper ordered frames of the cap's cell port 0.4's
  `grip_frames.py` corner order; under frame f a twist chord's item is the menu item equal to g_f M g_f^-1 by exact matrix
  equality, and a frame without one is refused with its reason. Setting a grip re-binds only the Jumble twist row's chords in the
  router (`Router::apply`), commits nothing and writes no record; cap 0 at frame 0 restores today's map (`GripCap`, J 1, K 2/5,
  L 3/9). Jumble off answers "Grip frames need Jumble".
- Shared chrome: `Model/GripFrame.cs` (cap, frame, admissible frames, ordered `Marks`, `KeyMeanings` with each chord's item and
  its cycle over the marks; `Next`, `Set`, `Changed`, `Dump`) is the read ADR-D01.74's key map draws on its keycaps;
  `Ui/GripMarks.cs` draws one mark for views and keycaps alike. `CorePlaces.PlaceCellPoint` places a cell point by the core's own
  projection, including Global's charts and Local's raw and slice panes.
- Shell: `D02/ShellGrip.cs`. `grip.select` (the picked cell at the current revision, or a named cap and frame) and
  `grip.frame.next` are draft rows in the D02 table's `grip` section (`grip.select`'s id is the command table's; the D02 row
  carries it). While Ctrl is held and the hover lies on the grip, the click twist (the frame's KeyJ item) outlines its moving set
  in palette entry 2 in both panes, with its cycle drawn at the cap's centre; release, focus loss or leaving the window clears it
  and nothing commits. Ctrl + click on the grip routes one KeyJ press (the key route, so the journal, the sweep and the
  correctness control see a press); a plain click stays B17's pick. No core change.
- Run: `d02_run.py --script grip-frame --state wj-ia-300`. J1 types both rows (cap 150, then the next frame).

**As built (ADR-D01.74's keycaps on the Grip frame; 27 November 2026, branch `claude/d02-port`).** The key map reads the
same `GripFrame` as the views.
- `KeyMapModel.Caps` takes the grip. Where the chrome leaves a twist chord to the live session (the key binds nothing in the
  hovered context, or binds a twist, and is no host key), its cap carries the grip's `GripKey`: the chord's item and its
  cycle over the numbered marks. The board stamps that cycle with `GripMarks.Direction`, the mark the views draw. Over
  Global, L stays the chrome's Local view. The header names the grip (`grip: cap 150, frame 1`).
- A frame change on the same cap marks each grip cap whose twist differs from the one the map last showed, with the
  corner mark of a key set change (`KeyMapModel.GripChanges`, context `grip`), until the map has been shown and closed;
  a mark not yet seen survives a second frame change. The cycles stay, since the marks turn with the frame. A cap the
  chrome takes (L over Global) is not marked. A grip on another cap drops the marks.
- Shell: `D02/ShellGripKeys.cs` hands `ShellGrip.Model` to the key map and dumps its grip caps and stamps (`keycaps`).
  Run: `d02_run.py --script grip-keycaps --state wj-ia-300`.

### ADR-D01 ports

ADR-D01 clauses ported into the D02 product shell outside the B steps. Each entry is its As-built note.

- **As built, ADR-D01.65's scene part (status track, 27 November 2026; Codex call 20261127T053634Z-672d825e):**
  - Core (ABI 10, no new number): style channel 3 (`SA2_STYLE_CHANNEL_STATUS`) carries one status byte per piece: bits 0 to 2 the status (1 protected, 2 conflict, 3 unchecked prefix, 4 preview, 5 stale), bits 3 to 5 the protection-kind mark (1 orbit, 2 position, 3 identity, 4 block), bits 6 and 7 reserved. `sa2_status_style_set` takes a 520-byte table: eight patterns and eight marks of shape, spacing, width, angle and RGBA. `sa2_status_drawn` reads back each pane's covered pieces. The shader draws the status after the user stages and the .25 error hatch, before the colour matrix; a conflict is never dimmed and breaks through focus, dim and hide. The host adds `m6h_set_status_style` and the status targets (orbit ids, slot occupants, a cell's 433 slots).
  - Design data: `tools/looklab/data/design/status-patterns.json` keeps the five statuses and four marks as separate tunable entries. Each status reads by shape as well as colour: protected a 45-degree hatch, conflict a cross-hatch, unchecked prefix dots, preview a dashed edge, stale horizontal stripes (moved off the diagonal the .25 error hatch uses). Marks: ring (orbit), dot (position), cross (identity), corner ticks (block).
  - Fed: protected and the four marks, from a B17 pick and the draft row `protection.pick.set orbit|position|identity|block|clear`. The marks name `ProtectionKinds` from the rail (`StatusPatterns.MarkCode` and `KindOf`); there is no second enum. Conflict and preview are fed from the held turn's moving set. With the sample on, a fed status with nothing to show (no protection, no held turn) draws the fixture's pieces instead.
  - Drawn, not yet fed: unchecked prefix and stale come only from `tools/looklab/data/samples/jumble/status-fixture.json`, shown by the draft row `status.sample.show on|off`.
  - Product script `status-patterns` (1280x720, wj-ia-300) checks the bytes sent, the table and both panes' coverage for each kind and the sample, and that clear and off leave no status byte. It picks sweep.json's first lattice hit, since pick.json's hits are off-lattice pieces with no slot.
  - Left over: setting `OrbitResidual.Protection` from the scene's protection so the rail's locks show it. The rail's orbits are sample data with no piece-to-rail-orbit mapping yet, so this is not a small step.
- **As built, ADR-D01.55's motion rules (status track, 27 November 2026; Codex call 20261127T073305Z-97697912, gpt-6.2-sol, effort max):**
  - Motion uses: `MotionUses` (`tools/looklab/app/Chrome/Model/MotionUses.cs`) lists each motion the shell and the shared chrome draw against its H-03 row, with its trigger (state or view) and kind (animated, static or held). Animated: the committed sweep (`jumble.sweep`), the queued sweep (`twist.queued`), the half-turn flash and B22's chart outline (`jumble.half_turn`), the wheel zoom and the raw chart clip (`view.transition`), the palette and in-window overlays (`dock.open`) and the sample strip edge (`strip.change`). Overlays, tints, ADR-D01.25's overlay, ADR-D01.65's patterns, the rail locks, the Jumble marks, `repair.step`, `cap.name`, the pending glyph and every result are static; `slice.turn` and the product's `reward.mark` are held. Overlay fades now run only when an overlay opens.
  - The engine commits before any motion: each sweep's dump row names its committed revision, the armed row's revision before it, and the first frame's revision, which equals the committed one with phase 0 drawn.
  - Twist queue (`MotionQueue.cs`, fed by `Shell.cs`'s key path and `D02/ShellMotion.cs`): a key twist with no sweep running commits at once. While a sweep runs, every head-changing key event (twist keys, `Ctrl+Shift+Enter`, `Ctrl+KeyZ`, `Ctrl+KeyY`, with their releases and repeats) waits in order and is sent through the live path when the sweep ends. Only a new press speeds up: the running sweep ends within `twist.queued` of its start, and each later sweep of the burst is shorter (110, 91.3 and 76.4 ms in the run), never below one 60 Hz frame. A single tap's own release never shortens its sweep (integrator fix to the Codex patch). A commit command, a session resume, Explore, focus loss and switching reduced motion on flush the queue in order with static motion.
  - Views: a wheel notch retargets the zoom, which eases over `view.transition` (260 ms) from the drawn value; a second notch multiplies the target (two notches reach 1.21). The raw chart clip eases the same way. The shell has no rotate click or view-reset step; rotation is the camera drag.
  - Drags (revision 6): the camera drag now turns by every pointer increment, including the movement before the three-pixel threshold, with no snap; pane splitters keep the ratio measured on release; the tuning sliders' value steps are field quantisation, not alignment. The slice names its certified twist only on key release, which a test pins; no drag changes an exact state.
  - Reduced motion stays off by default (`motion.reduced.set`, frozen row). Rewards: the appearance now stores `rewards=off`, set by the gallery's Milestone marks switch; it gates the gallery's rewards and the sample History's in-place first-orbit badge. Live D02 has no completion moment yet, and Jumble results appear in place for every outcome. No draft row was needed.
  - Product script `motion` (no `--state`, the solved jumbled puzzle): three quick keys queue and commit three revisions with shorter sweeps; two quick notches add up; after rewinding, the same keys under reduced motion are static and reach the same state digest. `sweep`'s expect keeps jumble.sweep's 220 ms for a single press.
  - The Python fixture suites now make their temporary folders under `work/` through one helper, which the Windows sandbox can reach.

**As built (ADR-D01.10's preview view, with what ADR-D01.11's commit bar needs; 27 November 2026, branch
`claude/d02-research`, gpt-6.2-sol call 20261127T084324Z-7bc36885).** A staged repair is a whole-only preview drawn in its
own pane; the main pair keeps the committed state.
- Core (ABI 10, no new number, as B18 and B22 added theirs): `sa2_preview_poses_set` binds a preview pose set (pose index,
  poses, lattice, outline) to the adopted revision and `sa2_preview_transform_set` the current step's motion. A pane with
  `SA2_PANE_PREVIEW` draws them through the existing shaders by re-binding the pose path's buffers (no shader change);
  `SA2_PANE_PREVIEW_BEFORE` draws the adopted poses with both transforms off. Preview panes answer no picks; the set survives
  recovery and is dropped by a newer adoption or a scene unload. Panes without the flags draw as before.
- Host (version 14, no new number; `renderer/bridge/host/src/preview.rs`): `m6h_preview_bind`, `_build`, `_scrub`, `_clear`,
  `_args`, `_moved` and `_report`. The build replays each letter on a copy of the head's poses, gives the exact digest at
  every prefix, and converts each prefix through the adoption's pose path; it commits nothing. Jumble off is refused.
- Shared chrome: `Model/RepairPreview.cs` builds a fresh preview from `RepairStage`'s staged word under a new id (settle
  interval, ADR-D01.9); `PreviewModel` gains whole-only (commit writes the whole word wherever the scrub is, rev6 .12),
  moved counts per step and in total; `PreviewPanel` in `Ui/SessionPanels.cs` draws the witness sequence, the counts, the
  orbit effects and the protection verdict; the frame's free view carries a binding chip "preview #id" and a close button.
- Shell: `D02/ShellPreview.cs`. The preview opens as a free view whose opening is a further pane after Global's and Local's;
  Global and Local show a pending chip only. `view.preview.scrub` steps (Left, Right), `preview.side.*` set the comparison
  (Both falls back to After when the slice split leaves no fourth pane), and preview steps animate on motion row
  `twist.preview`. Esc (`operation.preview.cancel`) or the close button clears the preview and the staged word. Ctrl+Enter
  (`operation.preview.commit`) or the commit bar commits through the existing route: one `m6h_invoke` of
  `jumble.twist.menu` per letter, in order, with `Turns::twist_letter`'s arguments; the head digest must then equal the
  preview's after digest. The palette cannot commit. One labelled entry per commit waits for CR-15 and K2.
- Script option: the draft row `preview.overlay.script.toggle` draws a marked overlay of the preview's moved set over the
  committed state in both main views; off by default.
- Settle interval (ADR-D01.9, ADR-D01.66, rev4's ADR-D01.11): a fresh whole-only preview starts the settle interval
  (`PreviewModel.Settle`, 0.4 s) as a replacement does, so a key-down never commits a preview the solver has not seen.
  While it runs, Ctrl+Enter is refused and the status line says why ("the preview has just changed"), and the commit
  bar shows the interval: its status reads settling, its settling lamp is lit (token `commit.status.settling`; the
  draft line and the commit bar share `SessionWidgets.SettlingLamp`) and Commit is off with its reason. A click on the
  bar's Commit does nothing then. All three go back to ready when the interval ends. The preview script restages the
  repair and presses Ctrl+Enter one frame later; the dump's `preview.settling` and `preview.bar` carry the check.
- Run: `d02_run.py --script preview --state wj-ia-300` (row 1 of cap 150: three letters, 4,637 pieces moved; the commit
  advances the head by three revisions to the preview's after digest).

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
  - the watching window starts without activation and is never topmost (section 1);
  - no protected file is in the diff.
- Owner is shown: the W-J state posed under the frame, with the Jumble cell and marks.
- **Record (integrator, 26 November 2026):** Fable read B1 to B6 at `7a2d3b16`. Six of the eight questions hold; questions 3 and 7 had findings.
  - C1-1 (question 7, major): the hand-back's later checks ran by frame time. After a long session open they could take the foreground back from an owner who had clicked the window meanwhile.
    - Adopted. A check that finds the shell back in front after another window had the front stops the checks for good: only the owner brings it back.
    - Not run: the W test of a real click during a 17-second open.
  - C1-6 (question 7, minor): behind a topmost foreground window the shell went to the top of the normal windows.
    - Adopted. It goes behind the launch's previous window, or to the bottom.
    - An Alt+Tab to the shell now stands, by C1-1's rule.
  - C1-2 (question 2, minor): two states can share both pose-array digests.
    - Adopted. The state digest joins the key, binding follows the adopted revision (B5's amended note), and a failed read unbinds (C1-5, adopted).
  - C1-3 (question 3, minor): a request at another revision did not make the shown result stale.
    - Adopted, with a console test.
  - C1-4 (question 3, minor), two parts:
    - A completed result that tried less than all is now an error, with a test. Adopted.
    - The empty word's cap and revision: deferred to B12, whose empty line names them ("Cap c, rev r: ...").
  - C1-7 (question 1, minor): `wj-ia-300`'s and both `nu-c` digests were unpinned.
    - Adopted. They are now constants in `tests/test_d02_samples.py`, which also checks ν against the correction's row.
    - Contact classes are checked at B15. The G3 state is recorded at B19.
  - C1-8 (question 6, nit): the dock sides and the Library panel are unconditional with `--frame`. Kept: they are ADR-D01's frame (B4a), not D02's.

**C2, after B13.**
- Fable checks:
  - the Q3.b test compares the whole screen;
  - no path orders, weights, pre-focuses or pre-stages a repair row;
  - no call to `Stage` lacks a user event;
  - blocked caps are undefined everywhere;
  - total N is never derived from caps;
  - research origin is refused at admission.
- Owner is shown: the cap map, the lists, a staged repair.
- **Record (integrator, 26 November 2026):** Fable read B7 to B13 at `0961f7cd`, by reading only. Four of the six questions hold; questions 1 and 6 had majors. Both majors are fixed before the next block.
  - C2-1 (question 1, major): the Q3.b test compared only the kept list, inside one synchronous loop, so it held by construction.
    - Adopted. The cancel goes through the host as the job layer will send it (`JumbleHost.CancelOlder` withdraws the job and cancels the release). The jobs' clock is held. The shell reads the whole screen (every region, `ScreenText`), lets frames pass, and the dump reads it again. `d02_run.py` compares the two reads.
    - A last `screen_control` step closes Local's open section under the same two reads: a negative control the run must count.
    - Run on wj-ia-300: 11 cancel steps and 92 cancels with 0 screen changes; the control was counted.
  - C2-6 (question 6, major): admission by origin was never called by a staging entry.
    - Adopted. `PreviewModel.Show`, `RepairStage.Stage` and `RepairStage.Type` take a required `Origin` and admit it themselves, with the session's own flag (`JumbleState.SessionKind`; the preview reads a session without one as solving). `StagedWord.Type` is internal.
    - A source pin in `tests/test_renderer_shell.py` names the only callers of `Admit`.
  - C2-2 (minor): a held digit staged a row per key repeat. Adopted: a pick is one plain press, with no repeat and no modifier.
  - C2-3 (minor): a list showing its error still picked. Adopted: `Digit`, `Enter` and `Stage` refuse a list with an error or one that is not completed.
  - C2-4 (minor): nothing pinned the exception's order. Adopted: `RepairListModel.Read` refuses rows out of (x, d) order, and `tests/test_d02_samples.py` checks every sample.
  - C2-10 (nit): an uncertain cap's null `blocked` threw. Adopted: its straddlers are pending.
  - The question-3 nit: only the list control makes a `UserEvent`. Pinned.
  - Deferred to the refinement round:
    - C2-5 (minor): section counts read "· 0" at a blocked cap and for a cut-off or cancelled result; the cap map's u reads pending where Local's border reads 231; a refused twist-check row would show Δn_h 0.
    - C2-9 (needs an experiment): a section rebuild may drop the repair list's keyboard focus, so the next digit goes to the session.
    - The question-5 nit: the shell reads `totals.json` as complete coverage without its coverage field.
  - Left as leads for CR-15 and K2:
    - C2-7: a staged span stays after the named cap or the revision changes.
    - C2-8: a span after other letters keeps the ΔN of the revision's own state. Reading ADR-D02.33, a pick starts a fresh preview.

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
- C3/C4 findings (27 November 2026), Fable's read at b1c0cf9b; each lead got its test first:
  - F1 (major, C3.2), confirmed: host test `style_held` failed first (the new revision's first frame had no fill).
    `m6h_set_piece_style` now takes the revision (ABI 10, before any release, no new number); the host holds a style
    for a revision handed out but not adopted yet and sets it at that adoption; an older revision answers
    unavailable, one never handed out invalid argument. The shell sends the outline and the fill for the adopted
    revision and, after a committed press, for the pending one. Left: the slice channel is still sent only for the
    adopted revision (its row is read per adopted revision), and presses committed outside `m6h_key_down` do not
    name a pending revision.
    - LF1 (27 November 2026), the slice and status channels: both are now also sent for the pending revision.
      `ShellTurn.NotePending` reads item 1's row at the head a committed press handed out (`Note` reuses that read at
      the adoption, so a twist still reads its row once); `ShellSlice` sends its inside set and `ShellStatus` its bytes
      for that revision, and the host holds them. `m6h_status_targets` also reads the handed-out head (ABI 15, no new
      number: the signature is unchanged). Host test `style_held`'s second case checks the core's slice and status
      against its adopted revision at every frame of a twist.
  - F2 (major as a checkpoint item, C3.9), confirmed: host test `a_pick_names_the_view_it_hit` failed first (the fake
    core answered the frame's first pane for a pick in Local); the fake now picks per pane, as the core's `pick_pane`
    does. `ShellPick` keeps the clicked view and drops a reply that `ViewPanes.Takes` refuses (the dump's
    `pick.dropped`); the `pick` run clicks twice in Local (`"in": "local"`) and checks each selection's view.
  - F3 (minor): not done in this step (budget); the four-state console replay is still owed.
    - LF1 (27 November 2026): no fault. `D02FourStateSessionAgainstRecords` replays wj-ia-300, wj-ia-end, deep-1 and
      deep-2 with one memory (commits, undos back, redos, a reopen): the first state equals its class record, each
      later state keeps every remembered class unless a touching domain kept it first, the colourings are proper, a
      state seen again draws as before, and the memory holds each id's first class and never the overflow. `Assign`
      is unchanged; the test is kept.

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
- C3/C4 findings (27 November 2026), Fable's read at b1c0cf9b:
  - F4 (minor), confirmed: `check_core.py --chart` on WARP failed first (after three channel changes evicted the
    frame's copy, the one-view pick was stale and the pane pick read the new channel). `pick_pane` now checks the
    copy's chart version, as `pick_plan` does.
  - F5 (minor): `hopf`, `ring-sheet`, `slice` and `raw` now also require one bound digest and revision (the dump's
    `bound`) and one Word panel digest across all their steps.
  - F6 (minor): by reading, not reproduced in a run (no script turns Jumble's mode off): `JumbleHost.Marks` returned
    before Local's raw caption while Jumble's mode was off. The caption is now added whenever Local draws the raw
    view, and `raw` reads it from Local's drawn border (the dump's `local_marks`), not from the model's constant.
    - LF1 (27 November 2026), reproduced in a run: no fault. The new script `raw-off` (the base puzzle, Jumble's mode
      off, no state) turns the raw view on and off with Home; `d02_run.py`'s check finds the caption on Local's
      drawn border exactly while the raw view is on, and no Jumble mark. `d02_run.py --script raw-off` passed.

**C5, after B29.**
- Fable checks:
  - the verdict words are exact, and cut-off, not-checked and NotApplicable are never a pass or a fail;
  - every research control invokes a row;
  - the synthetic sample is labelled.
- Fable also writes ten planted defects, sealed, added to `defects.json` only after B32's driver is committed.
- Owner is shown: the research pane.
- C5 findings (26 November 2026), Fable's read of B28 and B29; each fix has its test:
  - C5-1 (major): a null check verdict read as a pass; fixed in 099ee050 (`D02NullVerdictIsNeverAPass`).
  - C5-2: Runs and Claims worded a not-applicable run differently, and a run with no group got the scope "its run";
    fixed in 099ee050 (`D02RunLineIsTheClaimsLine`).
  - C5-3: a run its researcher cancelled says "the run stopped at its budget"; ADR-D02.45 and .46 have no words for
    it, so the words stay: freeze card. Never a Pass or a Fail (`D02CancelledRunIsNeverAVerdict`, 099ee050).
  - C5-4a: a Fail recorded before a cut-off needs an owner reading: freeze card.
  - C5-4b: dependencies dropped a missing input's source and cites and an input's sha256; fixed in 099ee050.
  - C5-5: the research rows were registered only with D02 on; fixed in 099ee050 (`ResearchRows.AddDrafts`).
  - C5-6: Save said "saved" without calling `research.bundle.export`; fixed in 099ee050.
  - C5-7: three research draft rows lived only in the sample; moved to `command-rows-d02.json` in 099ee050.
  - C5-8: keyboard reach of the pane: B32.
  - C5-9: the sample chip omitted Research and the run expectations did not assert it; fixed in 099ee050.
  - C5-10: sample record file names shared with real archive records; renamed in 099ee050.
  - C5-11: a carried copy or save did not name its evidence class; fixed in 099ee050.

**C6, after B33.**
- Fable checks:
  - caught defects are counted honestly;
  - rules not run are reported as not run;
  - the subagent stayed inside its cap;
  - section 4 still matches the code.
- Owner is shown: the J1 session and the report.

**Refinement, after C6, then the freeze (owner, 26 November 2026).**
- Fable's refinement scan reads the built product once more and gives the one build round its list. Before it, the build runs no
  stop; after the build round come the two Astra rounds, then D02's freeze card.
- Visual check (owner, 26 November 2026, 21:59 UTC): the scan goes through every D02 view one at a time: Global with each chart
  preset (the 20 rings, the ring sheet, the Hopf rings), Local, the cut slice, the raw view, the tints and the overlays. For each
  view it asks whether a researcher can see the structure the research needs (contact classes, caps, the cut, what moves, what
  never hides, the ν band), and whether anything is drawn wrong, unreadable or misleading.
- With the core functions (owner, 26 November 2026, about 22:10 UTC: on the research side, one main purpose of Script is to
  solve visual problems): Fable judges each view together with piece filtering and Script, not the default drawing alone.
  - Piece filtering: focus layers with `=` expressions (`PieceFilter.cs`, ADR-D01.46, G21), Query and Piece sets.
  - Script: `.6cs` read programs through the shell's editor and runtime; every research row is callable from a script
    (ADR-D02.1).
  - Fable writes the filters and short read-only scripts a researcher would write and runs them in the product window: for
    example the straddlers of a named cap, the off-lattice pieces of height at least k, a repair row's moving set, one domain
    against another. It checks that both views show each result readably: focus, dimming, tint, overlay and counts.
  - A problem a filter or a script solves well counts as solved when the path works and a researcher can find it. A broken or
    missing path is functional: a script result that cannot reach the focus layers, tints or overlays, an `=` field the research
    needs that is missing, a result that is not drawn, or one that cannot be read. A route the ADRs do not name goes on the freeze
    card as a proposal and is not built silently.
  - Fable's working filters and scripts are kept as examples for B32 and B33's researcher run.
  - The boundaries hold: a program reads state or prepares previews (LQ21's capability separation), and the human-solve boundary
    holds in solving sessions.
- Output: every visual problem found, each marked `functional` or `taste`.
  - Functional: the structure cannot be seen or operated (rotate, pick, focus), is drawn wrong, or misses the frame-rate rows. All
    are fixed in the one build round before the two Astra rounds. The base puzzle's sticker pick (ADR-D01.40, host
    `resolve_sticker`) counts here.
  - Taste: colour values, materials, motion and polish. They are recorded for K2's art line (UI/UX freeze rounds 3 and 4) and are
    not fixed in D02.
- The freeze card shows the owner the list once, with what was fixed and the proposed routes, to confirm. The owner is not asked to look for problems.
- The freeze card also lists, for the owner's acceptance with ADR-D02: the draft command rows (`camera.link`, Local's auto-framing
  switch, `jumble.seat.hold`, XM1 and G6, `research.record.read`, the researcher-name write, `tuning.open` and the rest of
  `command-rows-d02.json`), the design slots `tint.jumble.off_lattice` and `line.jumble.overlay` (each adds a name to ADR-D01's
  frozen round-1 slot list), and the eleven glossary additions of ADR-D02 section 15. Until then the draft rows and fallbacks stay.
- The freeze card also says that ADR-D01.77's instruments without roles until now (arrows, orientation glyphs, set-kind glyphs
  and block marks) are registered with revision 4's values and neutral placeholder drawings (`InstrumentPlaceholders`,
  `InstrumentGlyphs`): `arrow.*`, `orientation.glyph.*`, `set.kind.*`, `block.mark.*` and `block.outline.*`, the last under
  block marks. D02 places none of them; ADR-D03's canvas places them, and the art line draws them.
- Proposal, not built: where D02 already shows a set's kind in words (the Focus layer editor's kind buttons in
  `PieceFilterUi.cs` and the focus layer's rows), the `set.kind` glyph could replace or join the word. This serves K2's
  fewer-words item.

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
- **The owner's window rule** (26 November 2026, said in the design-system ADR line and relayed here): the watching
  window never sits on top and never takes focus. The integrator wrote it into section 1, B1 and B4; gate rows keep
  their own foreground rules.
- B1 waits for the owner's go.
