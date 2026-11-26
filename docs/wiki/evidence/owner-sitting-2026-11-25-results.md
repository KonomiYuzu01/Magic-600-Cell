---
id: owner-sitting-2026-11-25-results
type: evidence
status: verified
visibility: public
summary: The owner-machine sitting of 25 November 2026, on builds from main bb416a2c (whose code equals LP4's candidate 0208e78d), on the owner's RTX 4070 Laptop GPU. The greybox flow (LP4-3) and three of CT-EV4-05's four legs ran on screen, driven by the integrator through Windows input, and every step worked. No twist is visible in the default view. U4: the renderer gate is met in W-J (60 fps, p99 16.9 ms); W3 has 2 of 3 runs. U7: the stutter under runtime loss is unchanged (38 to 58 fps, p99 55 to 61 ms). LX-2 and LX-4 read not-met. Their latencies over confirmed frames hold, and all 152 LX-4 cells meet their bars, but 15 to 18% of the measured runs' frames have no confirmed present (2 to 6% on 21 November), so E2 fails on keys without a confirmed frame. L completion and L diagnostics are about 50 ms slower; on 26 November 2026 headless timing traced that to the editor services reading each L buffer again, and the fix removes most of it. The confirmation gap is a sampling race, traced on 26 November 2026 in presented-gap-2026-11-26. Reading (a)'s halves and the lattice replay did not run.
related: [owner-decisions-2026-11-25, owner-decisions-2026-11-24-lp4-lp5-exit, owner-decisions-2026-11-22-c10-lp2-exit, owner-decisions-2026-11-22-lx-bars-freeze, c9-sitting-results, presented-gap-2026-11-26]
supersedes: []
claims:
  - {id: os1125-sitting-agreed, evidence_kind: decision, path: docs/wiki/decisions/owner-decisions-2026-11-24-lp4-lp5-exit.md, line: 90, sha256: 2fe0644a17ffabbb38978b7b39d7bcd16a1abd834dfacd3577f6a54c7c628177, checked_at: 2026-11-25}
  - {id: os1125-sitting-shape, evidence_kind: decision, path: docs/wiki/decisions/owner-decisions-2026-11-25.md, line: 48, sha256: b6a95b55e0a8679c8851c08fa2012cd549c2bab23010a7b6cd2da6fce395ff16, checked_at: 2026-11-25}
  - {id: os1125-lattice-waits, evidence_kind: decision, path: docs/wiki/decisions/owner-decisions-2026-11-25.md, line: 59, sha256: b6a95b55e0a8679c8851c08fa2012cd549c2bab23010a7b6cd2da6fce395ff16, checked_at: 2026-11-25}
  - {id: os1125-owner-ev4-05, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/owner-ev4-05.json, sha256: 7454b6e3e341356f2e4636ba9b25a627ef9c4c2f7442eee1229a540c562c3968, checked_at: 2026-11-25}
  - {id: os1125-owner-greybox, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/owner-greybox.json, sha256: f37aabd839a63f8151af375b051e3887f2b34ff22152014ba716175e7a0eb1d3, checked_at: 2026-11-25}
  - {id: os1125-integrator-greybox-ev4, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/integrator-greybox-ev4.json, sha256: fa20bd4050c0f9a9285d9fb285a8a06056107ae45323cd2cfd140ed4389ddc1f, checked_at: 2026-11-25}
  - {id: os1125-window-default-size, evidence_kind: source, path: renderer/shell/project/project.godot, line: 13, sha256: ef6bcd1aa175f067e874e729359e7e8d9aee5b1b5a5e2b4917327bf598fe8fc7, checked_at: 2026-11-25}
  - {id: os1125-editor-panel-size, evidence_kind: source, path: renderer/shell/project/Editor/EditorPanel.cs, line: 87, sha256: 9ddc7a9b9891b5456319dd4e7e0a4e7e61a9d3e3fa5c23a29a18f6f9f002f358, checked_at: 2026-11-25}
  - {id: os1125-unattended-log, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/unattended.log, sha256: 9ce9bc16acbe8de1df51f11fb13fce2e61342ecc85fe31e0da0e9cb36d09fc5a, checked_at: 2026-11-25}
  - {id: os1125-u4-wj-s4-gate-met, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/S4-summary-s1125.json, sha256: 8e86c2c581358ee42f5ed79da397f10b078dd86649ab7fe75a277abc4f1c3734, checked_at: 2026-11-25}
  - {id: os1125-u4-wj-i-a-gate-met, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/I_a-summary-s1125.json, sha256: 565359837faf55d42cfbd7c9dba516631f50201df8e4dbb5b1cabb91788ee6e9, checked_at: 2026-11-25}
  - {id: os1125-u4-wj-i-b-gate-met, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/I_b-summary-s1125.json, sha256: e62949bed7aa1ff0eda9e0638bcbc0efe8cfcf58f1dbe3ce517de640054bc98f, checked_at: 2026-11-25}
  - {id: os1125-u4-w3-insufficient-runs, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/w3-summary-s1125.json, sha256: dfeb57046a97302ed57e1652ee5d09fd6e970973a256c194224c23ecb44a1d67, checked_at: 2026-11-25}
  - {id: os1125-u4-w3-run2-refused, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/w3-2-refusal-s1125.json, sha256: 90141976f82aa8955ac0133b70ae75b242696194f87f9e6d2815431007f63aa5, checked_at: 2026-11-25}
  - {id: os1125-console-window-change, evidence_kind: source, path: rust/c600-platform/src/lib.rs, line: 52, sha256: f5b5eb2d57d2948d39913b054d10fc39dd26c70716768edd066f3c0d63f49c95, checked_at: 2026-11-25}
  - {id: os1125-u7-lx2-faults-agree, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/u7-s1125-faults.json, sha256: f5278741d83e3320c942bc8f849887a9a864a8d48c576b870b410ad0bae73d11, checked_at: 2026-11-25}
  - {id: os1125-u7-lx2-not-met, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/u7-s1125-lx2.json, sha256: 854f4c729a9d22e2c0fb394ea3a71fdc8adb7d2861c6f4499b44e0e61f689346, checked_at: 2026-11-25}
  - {id: os1125-u8-e2-not-met, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/u8-s1125-e2.json, sha256: e427cf74aa15392392e606a90dfa418a410ab67b0dec07481ddcd66e6572d64b, checked_at: 2026-11-25}
  - {id: os1125-u8-lx4-not-met, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/u8-s1125-lx4.json, sha256: eedc660489ee9435eb992e7f58a17585fb3ba21c391a88fd72050e08295fb572, checked_at: 2026-11-25}
  - {id: os1125-presented-share-tonight, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/presented-share-s1125.json, sha256: f61ca64965d6da462f4d489aee501c4f4384bd3a0d49ff1dc8b104f37e205a71, checked_at: 2026-11-25}
  - {id: os1125-presented-share-c9, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261125/results/presented-share-c9-20261121.json, sha256: 1f13adc7dba95db68ff7f19f9b9e100458001705355e52a48ffb9ae652dd7737, checked_at: 2026-11-25}
  - {id: os1125-chrome-off-without-env, evidence_kind: source, path: renderer/shell/project/Shell.cs, line: 135, sha256: e2210b163390226b76d0e4969067dcd156fa4271da1bea56b4aa866d50d7dade, checked_at: 2026-11-26}
  - {id: os1125-lx4-l-slowdown-found, evidence_kind: performance, path: work/experiments/lx4-latency-20261126/README.md, sha256: 715980aa091b970ab4c14df6cf9c464e26a6024674367d5f746f3a7a3caea876, checked_at: 2026-11-26}
---

# Owner-machine sitting of 25 November 2026

Item 6 of the [LP4 and LP5 exit decision](../decisions/owner-decisions-2026-11-24-lp4-lp5-exit.md) agreed one sitting on
the owner's RTX 4070 Laptop GPU with five parts:
- the stutter fix's device re-run;
- the owner-machine halves of reading (a) of the [C10 and LP2 exit](../decisions/owner-decisions-2026-11-22-c10-lp2-exit.md);
- LX-4's editor bars on LP4's candidate;
- CT-EV4-05's Windows half;
- the greybox flow.

It ran on 25 November 2026, in the shape that the owner set that day
([decision](../decisions/owner-decisions-2026-11-25.md), item 3):
- attended: CT-EV4-05's Windows half and the greybox flow;
- unattended: the stutter re-run and LX-4's editor bars;
- not run: reading (a)'s halves.

The records are in `work/experiments/owner-sitting-20261125/`
([README](../../../work/experiments/owner-sitting-20261125/README.md)).

**Builds.** Every build was made from main `bb416a2c`. Its code equals LP4's candidate `0208e78d`: the two differ only
in `docs/`.
- Product build: `c9-product-a9d908ad262ee306`, made by `renderer/shell/build_product.py`.
- Route-B scene build: made by `renderer/acceptance/prepare_host.py --route surface`.

## What ran and what did not

| Part | Ran | Result |
| --- | --- | --- |
| Greybox flow (LP4-3) | Yes, on screen. The integrator drove it after the owner's first run could not reach the panel's controls | Every step of write, check, activate, run, preview and commit worked |
| CT-EV4-05, Windows half | Three of its four legs | IME composition, English text and Ctrl+Z in the code field neither twist nor undo the session. The held-key-across-focus-loss leg was not run |
| Renderer gate reference for U8 (U4) | Yes, unattended | W-J S4, I_a and I_b are met (60.0 fps, p99 16.94 ms). W3 is `insufficient-runs`: 2 of 3 runs were valid, and both met |
| Stutter fix's device re-run (U7) | Yes, unattended | The stutter is unchanged: 38 to 58 fps, p99 55 to 61 ms, in the faults that lose the runtime. LX-2 reads `not-met`, because 215 of 1,661 keys have no confirmed frame |
| LX-4's editor bars on LP4's candidate (U8) | Yes, unattended | All 152 cells meet their bars over the samples with a confirmed frame. The verdict is `not-met`: E2 fails, with 92 of 602 keys lacking a confirmed frame, and the gate reference does not count. L completion and L diagnostics are about 50 ms slower than on 21 November |
| Reading (a)'s owner-machine halves | No | The harnesses do not exist (below) |
| LP5's lattice replay | No | It waits for the owner's word ([owner, 25 November 2026](../decisions/owner-decisions-2026-11-25.md), item 4). The release test binary is built, and `work/sitting-1125/start-lattice.ps1` in the sitting's checkout starts it |

## The attended part

**The owner's two runs (17:16 to 17:20 UTC).** The owner ran steps 1 and 2 of the step sheet, but saw no text and no
turn on screen. Both are explained:
- The shell's window opens at 1280 × 720 (`renderer/shell/project/project.godot`, stretch mode disabled). The
  editor panel is 720 × 860 pixels at the window's top-left corner (`renderer/shell/project/Editor/EditorPanel.cs`).
  Its palette, its Invoke button and its status line therefore lie below the window's edge, and the panel covers
  most of the puzzle. The step sheet did not say to enlarge the window.
- No twist can make a visible change (below).

Records:
- **Step 1:** 5 twist key rows, the correctness control passed, and the foreground was lost (Alt+Tab). Three of the
  twists (J, K and L) came within 1.5 s. The logs do not show whether the code field had the focus then, so this run
  says nothing about CT-EV4-05.
- **Step 2:** no word committed. One `session.undo` stepped back the fixture's last twist, so the correctness control
  failed, as it does for any undo (`renderer/shell/README.md`).

**The integrator's run (17:27 to 17:37 UTC).** The owner asked the integrator to find out and do it. The integrator
launched the shell with `--resolution 2400x1400`, which shows the whole panel beside the puzzle, and drove it with
Windows input through computer use, without the owner's hands.

1. **Write.** Editing the default macro's body to `return true;` showed one diagnostic,
   `5:12 LG2:type.mismatch the result: Bool where Word is expected (LG2.1)`, and the status line read
   `refused: 1 diagnostics`. Changing it to `return word[+1, -2, +3];` cleared it.
2. **Check.** "Check the macro" answered `checked: mine.shown is a candidate (dd499df1ea8d); Activate makes it the
   active version`.
3. **Activate.** "Activate the checked macro" answered `activated: tool.mine.shown runs dd499df1ea8d`, and the
   palette gained the row `mine.shown (tool.mine.shown; Ctrl+Shift+KeyR)`.
4. **Run.** Ctrl+Shift+R, with the row selected and the code field without focus, answered
   `the macro returned word[+1, -2, +3] (process); it is the draft`. The run ran in the language runtime's own process.
5. **Preview and commit.** Ctrl+Enter, then Ctrl+Shift+Enter. The journal gained one `operation.execute`, and the
   frames drew engine and publication revision 26.
6. **Twists.** J, K and L, pressed outside the code field, committed three `twist.apply` rows. The frames drew
   revisions 27 to 29.
7. **CT-EV4-05's text legs**, with the code field focused:
   - **IME.** Microsoft Pinyin, in Chinese mode, composed `j'k'l'JKL` from j, k, l and Shift with J, K and L.
     Escape cancelled the composition.
   - **English text.** In English mode, `jklJ` was typed into the field.
   - **Ctrl+Z.** In both modes it undid only the field's text.
   - **The record:** no `key` row and no journal row was added by any of these. The key count stayed at 3 and the
     journal at 27 twists and one word.
8. **Close.** The correctness control failed, as documented for any change that is not a twist: its replay knows
   only twists, and the committed word is one. The foreground was kept throughout.

**Not shown on screen.** The preview draws nothing on the view, as LP4-3 records. The panel's status line does not
report a preview or a commit.

**What this run is.** The input was real Windows input to the shell's window, through the window manager and the
IME, but it came from the integrator and not from the owner's hands. The panel's texts were read from window captures,
which are not published. The held-key leg needs a key held across Alt+Tab, which the integrator's tools cannot do, so
it was not run.

## Finding: no twist is visible in the default view

Neither the committed word nor the twists changed the puzzle's picture: a screen capture of the puzzle area before and
after two twists differs in no pixel by more than 30 (the sum of the RGB differences).

The code says why (a source reading, not a GPU capture):
- **The colours do follow the state.** Each adoption hands the session's current labels to the core
  (`renderer/bridge/host/src/lib.rs`, `rust/c600-render/src/backend.rs`). The core re-uploads them for the frame's
  engine revision (`renderer/core/src/product.cpp`), and the shader colours each sticker by its label
  (`renderer/core/shaders/draw.hlsl`).
- **Every twist the keys can make acts at the grip's cap 0.** The grip is fixed there, because `grip.select` and
  `grip.rebind` are not built. The session fixture's 24 presses act there too.
- **Cell 0 is hidden.** It projects well inside the polytope, behind the outer cells, and the shell never moves the
  camera.

A CPU rasterisation of the default view, an estimate made for this reading, puts none of the 4,604 slots that those
two primitives move on screen. No test checks that a twist changes what the screen shows. Until the grip can move or
the camera can turn, a solver in the greybox sees no effect of a twist.

## Reading (a): not run

None of reading (a)'s halves ran. The post-freeze on-screen line that the
[C10 and LP2 exit](../decisions/owner-decisions-2026-11-22-c10-lp2-exit.md) names, which builds their harnesses and
RPI.77's quarantine, has not started. On `bb416a2c`:
- the shell has no picking and no preview on screen;
- no user stages are attached;
- nothing injects a device loss or a device fault;
- the quarantine exists only headless, in `c600-display`;
- the shell has no tool views;
- there is no OS-level file-access trace, no fault seam in the shell's host loop, no gesture producer and no
  cross-process comparison.

Two tests touch LX-2 in part: CT-LG4-08 and CT-ONE-20. U7's run gives their LX-2 parts as references only. Their
other parts (a direct kernel binding, and a pending preview at the runtime's death) do not exist. Nothing here counts
any of these halves as passed.

## The unattended part

The owner started the chain in an administrator PowerShell and went to bed. It ran from 17:47:53 to 22:20:07 UTC:

    & tools\perf\c9\unattended.ps1 -Steps U4,U7,U8 -SceneBuild <scene build> -ProductBuild <product build> -Suffix s1125

Its log is `results/unattended.log`. Each measured step waited until the owner had been idle for five minutes, and no
step was stopped by input. Nobody watched the screen.

### U4: the renderer gate on the scene build

| Scene | Valid runs | Verdict | Pooled fps / p99 (ms) | VRAM peak (MB) |
| --- | --- | --- | --- | --- |
| W-J S4 | 3 | **met** | 60.0 / 16.94 | 216 |
| W-J I_a | 3 | **met** | 60.0 / 16.94 | 216 |
| W-J I_b | 3 | **met** | 60.0 / 16.94 | 216 |
| W3 | 2 of 3 | **insufficient-runs** | 60.0 / 16.92 | 209 |

The finalizer refused W3's second run: "A checked present step does not contain exactly one trace entry"
(`results/w3-2-refusal-s1125.json`). Both other W3 runs met the gate. The figures match the C9 sitting's: p99 16.95,
16.95, 16.95 and 16.93 ms.

### U7: the stutter fix's device re-run

The build includes `2737153c`, which starts the language runtime's child process without a console window
(`CREATE_NO_WINDOW`, in `c600-platform`). U7 ran LX-2's seven faults with the injection child. The analysis
(`results/u7-s1125-lx2.json`, `lx2.py`, exit 3) reads **not-met**. Conditions 1 and 2 are met: the faults record and
the sessions agree. The correctness control passed, and the foreground was kept throughout.

| Fault | Gate fps / p99 (ms) | 21 November | E2 keys without a confirmed frame | E2 | Echo samples without a confirmed frame | Echo |
| --- | --- | --- | --- | --- | --- | --- |
| alloc_metered | 59.8 / 19.6: met | 59.8 / 19.4 | 28 of 111 | **insufficient** | 26 of 111 | **insufficient** |
| recursion_metered | 60.0 / 17.0: met | 59.7 / 17.1 | 2 of 110 | **not-met** | 1 of 112 | **met** |
| loop_metered | 60.0 / 17.0: met | 60.0 / 16.9 | 25 of 111 | **insufficient** | 27 of 209 | **met** |
| forced_kill | 57.1 / 59.3: known limit | 57.2 / 52.2 | 74 of 150 | **insufficient** | 23 of 110 | **insufficient** |
| alloc_ceiling | 46.9 / 54.8: known limit | 46.7 / 54.1 | 17 of 348 | **met** | 24 of 110 | **insufficient** |
| recursion_native | 38.4 / 55.5: known limit | 39.2 / 58.8 | 34 of 595 | **not-met** | 10 of 110 | **met** |
| loop_native | 57.7 / 61.1: known limit | 57.6 / 53.2 | 35 of 236 | **not-met** | 46 of 203 | **met** |

**The stutter is unchanged.** In the four faults that lose the runtime, the gate reads 38 to 58 fps with p99 55 to
61 ms. On 21 November it read 39 to 57 fps with p99 52 to 59 ms. So the console-window change does not remove the
stutter. The logs cannot show whether a console window opened: the run row reads "foreground throughout", and so did
21 November's run, made before the change.

**What fails is the count of confirmed frames, not the latencies.**
- E2 counts a key without a confirmed frame as a failed key (owner, 20 November 2026, item 2). Tonight 215 of the
  run's 1,661 keys have none. On 21 November it was 10 of 1,741.
- Over the keys with a confirmed frame, the latencies look much as before:

  | | p50 (ms) | p95 (ms) | p99 (ms) | Keys over 100 ms |
  | --- | --- | --- | --- | --- |
  | Tonight | 67.4 | 82.6 | 115.2 | 18 |
  | 21 November | 67.5 | 82.5 | 98.7 | 9 |

  These figures are not the bar's reading: they leave the failed keys out (`key_to_frame.py` over the run).
- The echo bar holds on every fault's samples with a confirmed frame (p95 at most 92.3 ms, p99 at most 94.1 ms).
  alloc_ceiling's owner exception (item 5) names 21 November's run only, so its 86 timed samples read `insufficient`.

### U8: LX-4's editor bars on LP4's candidate

U8 ran LX-4's fixtures for about 101 minutes with Jumble off, then about 101 minutes with Jumble on. Both runs are
valid. The analysis (`results/u8-s1125-lx4.json`, `lx4.py`, exit 3) reads **not-met**, for two reasons:
- **E2 is not met in these runs** (`results/u8-s1125-e2.json`). 92 of the 602 keys have no confirmed frame, so the
  p95 has no number. On 21 November it was 1 of 602. Over the 510 confirmed keys alone, p95 is 82.7 ms and p99 is
  115.6 ms, again not the bar's reading.
- **The gate reference does not count.** It is U4's W3 summary, which has only two runs and comes from the scene
  build, not the product build. Item 2 of the [LX bars decision](../decisions/owner-decisions-2026-11-22-lx-bars-freeze.md)
  accepts a same-night W3 summary only once it is entered as accepted. With two runs, it was not entered.

All 152 LX-4 cells meet their bars over the samples with a confirmed frame. The worst cell of each kind, p95 / p99
in ms (bar in brackets):

| Kind | S | M | L |
| --- | --- | --- | --- |
| Echo | 78.5 / 82.7 (100 / 100) | 78.2 / 82.5 (100 / 100) | 78.3 / 78.7 (100 / 100) |
| Syntax | 94.6 / 99.1 (100 / 100) | 94.9 / 99.4 (100 / 100) | 95.0 / 99.2 (100 / 150) |
| Stale | 77.6 / 82.3 (100 / 150) | 78.0 / 82.3 (100 / 150) | 77.7 / 81.8 (100 / 150) |
| Completion | 94.3 / 98.7 (100 / 150) | 94.3 / 98.4 (100 / 150) | 144.7 / 182.0 (150 / 250) |
| Diagnostics | 94.3 / 99.2 (150 / 250) | 94.3 / 98.7 (200 / 300) | 177.8 / 215.6 (500 / 750) |
| Rerun | 126.7 / 138.3 (250 / 400) | 126.6 / 143.2 (250 / 400) | 160.8 / 174.3 (500 / 750) |

W's dependents read 210.8 / 225.4 ms against 1,000 / 1,500 ms. The confirmation gap, counted apart and not judged
(item 10), holds:
- 1,340 of the 10,800 S, M and L samples, against 14 on 21 November;
- 100 of the 600 W samples, against 2.

**L completion and L diagnostics are about 50 ms slower than on 21 November.** Every other kind and size is within a
few milliseconds of 21 November.

| Kind | Tonight, p50 (ms) | Tonight, worst p95 (ms) | 21 November, p50 (ms) | 21 November, worst p95 (ms) | Bar p95 (ms) |
| --- | --- | --- | --- | --- | --- |
| L completion | about 141 | 144.7 | about 90.5 | 93.8 | 150 |
| L diagnostics | about 159 | 177.8 | about 107.6 | 110.6 | 500 |

The rise is the same in every series and with Jumble on or off. Both still meet their bars, but L completion has only
5 ms of headroom. Between 21 November's candidate `013900ff` and `bb416a2c`, 454 files under `rust/` changed (LP4's
and LP5's work), and so did the LX-4 library in `renderer/bridge/lx4`.

**Update, 26 November 2026: the cause, found headless.** LP4-3 moved LX-4's library onto EV1's editor services
(`rust/c600-editor`). Their completion read the buffer again: a parse, two lexes, and a name index that scanned every
binding of the file for each name it resolved, about 20 ms on L alone. Their diagnostics parsed and lexed the buffer
once more before the check. Since then the index looks names up by owner and by name, and a document shares one parse
between completion and diagnostics; no answer changes. In headless timing of U8's fixtures through LX-4's C interface,
the fix removes about 55% of the extra L completion time and about 63% of the extra L diagnostics time, and L syntax
keeps its path ([records](../../../work/experiments/lx4-latency-20261126/README.md)). The rest is the new
completion's own work: one more parse, a lex and the index. Whether the fixed L completion and diagnostics land on
21 November's frames on screen waits for the next U8 run.

## Finding: far fewer frames are confirmed as presented

The shell logs each frame it hands to the core (`frame`), and each frame the core confirms as presented through its
own swap chain's frame statistics (`presented`). A key or a sample whose frame has no `presented` row has no
confirmed display. `presented_share.py` counts these frames per run (`results/presented-share-s1125.json`,
`results/presented-share-c9-20261121.json`):

| Run | Build | Window | Frames without a `presented` row |
| --- | --- | --- | --- |
| U7, 21 November | `c9-product-2efda50d…` | default, 1280 × 720 | 6.0% |
| U8 Jumble off, 21 November | `c9-product-2efda50d…` | default | 2.3% |
| U8 Jumble on, 21 November | `c9-product-2efda50d…` | default | 2.1% |
| The owner's step 1, tonight | `c9-product-a9d908ad…` | default | 35.2% |
| The owner's step 2, tonight | `c9-product-a9d908ad…` | default | 40.1% |
| The integrator's run, tonight | `c9-product-a9d908ad…` | 2400 × 1400 | 0.06% |
| U7, tonight | `c9-product-a9d908ad…` | default | 17.8% |
| U8 Jumble off, tonight | `c9-product-a9d908ad…` | default | 16.6% |
| U8 Jumble on, tonight | `c9-product-a9d908ad…` | default | 15.4% |

The skipped frames are mostly single frames: the core's presented sequence steps by 2. Within one run, the share
changes over time. For U8 with Jumble on it ranged from 7% to 21% per five minutes.

The cause is not established. What the records rule out or leave open:
- **Ruled out: the launch and the capture.** Both nights used the same launch command and the same frame rate, and
  PresentMon captured only the shell. W3's records show the same display mode and the same driver.
- **Ruled out: the frame chrome.** The chrome added to the shell since 21 November (`ShellChrome.cs`) is off in every
  run here, because `M600_CHROME` is not set.
- **Open: the code.** Tonight's build also differs in the host library, the shell's event log and LP4's editor.
- **Open: the window and the desktop.** On the same build, the 2400 × 1400 window lost almost no frames, while the
  default window lost 15% to 40%. The harness controls neither the window's size nor what lies under its transparent
  area.

21 November's product build is still on this machine. Running it and tonight's build back to back, with the same
window and the same desktop, and then tonight's build at both window sizes, would decide. Each needs a quiet machine
and the foreground.

Follow-up, 26 November 2026: [presented-gap-2026-11-26](presented-gap-2026-11-26.md) traces the gap to a sampling
race that the window and the desktop decide, not the build, and records the fix in the core.

## What it does not show

- No result for other hardware, resolutions or refresh rates, or long sessions.
- The integrator's run is not the owner's: it shows what the shell does with Windows input, not what a solver
  experiences.
- The twist-visibility reading is a source reading with an estimate, not a capture of the core's buffers.
- Why fewer frames are confirmed, and why L completion and L diagnostics are slower: neither cause is established.
- Whether a frame without a `presented` row was shown late, shown by the next frame or not shown at all.
- No owner phase pass. Under PHS.66 these records are evidence for the owner's reading, never a pass on their own.
