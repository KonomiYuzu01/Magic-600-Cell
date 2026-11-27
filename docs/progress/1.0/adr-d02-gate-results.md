# ADR-D02 gate results (B19 and B24)

The gate rows of the ADR-D02 build (`docs/progress/1.0/adr-d02-build-framework.md`, B19 "Gate sitting 1" and B24 "Gate
sitting 2"). Each row is one command. The limits are the renderer selection gate's (`docs/wiki/decisions/renderer-candidates.md`):
an average of at least 30 fps, a 99th-percentile frame time of at most 33.3 ms and a peak VRAM of at most about 7 GB
(read as 7.0 GiB), at native resolution with no frame generation.

**Status: both sittings run on 27 November 2026; all eleven rows pass and the occlusion run is valid.** One owner
sitting ran them back to back (section "The sitting of 27 November 2026"). The occlusion run's first record called it
invalid through a checker fault, and the fixed checker scored the same run again (its row, below).

**Under review since 27 November 2026, 22:13 UTC.** Astra review round 1 (`docs/progress/1.0/adr-d02-astra-round-1.md`)
found checker gaps that question what these records establish: D02-12 to D02-18, D02-23 and D02-24. The result above
is recorded but not final until the checkers are fixed and the affected rows are rescored from their saved observations
or rerun.

## How a row runs

- `renderer/shell/gate_d02.py --row <id>` launches the product through `d02_run.py`'s launcher and player on the row's
  state's session fixture, with the event log and `M600_D02_GATE=1`. The row's script
  (`renderer/shell/d02_scripts/gate-<id>.json`) sets the views, then one repeat block runs the motion for warm-up plus
  window seconds (10 s plus 60 s by default) as quiet steps, which write no screen dump.
- `tools/perf/d02/gate_row.py` reads the run's `events.jsonl` and prints
  `D02_GATE row=<id> panes=<n> view_vertex_evals=<v> fps_avg=<x> p99_ms=<y> vram_gb=<z> pass=<bool>`.
  - Frame times: the gaps between the presentations the core confirmed (`presented` rows, `display_qpc`).
  - Warm-up: everything before the last 60 s of presentations is dropped. That covers the launch, the state's open, the
    view setup and the first 10 s of motion.
  - Panes: the pane count of the frame rows in the window.
  - `view_vertex_evals`: the core's own pipeline statistics. Every frame row carries `sa2_counters` on a gate run, and
    the value is the mean per reported frame over the window.
  - VRAM: the shell process's dedicated GPU memory at its peak in the window. It is sampled every 0.25 s from outside
    through the GPU Process Memory counters (Dedicated Usage, the figure Task Manager shows) into `gate-vram.jsonl`.
- `gate.json` in the run folder holds the line, the record and the run's validity. A run is invalid (exit 3, never a
  pass) in three cases: its window's pane count is not the row's, a sweeping row committed no twist in the window, or
  the owner's window rule was broken.
- Sweeps: each sweeping row (G1, G9, G4, G6, G8) selects its grip cap with `grip.select` before the repeat block, so the
  router's twist keys turn that cap. The scripts name cap 2, which is admissible at `deep-2` and `wj-ia-end`. At another
  state (a dry run) `gate_d02.py` moves the selection and the cap naming to an admissible cap of that state, read from
  its sample `rim.json`: the lowest admissible cap with its own `cap-<n>.json`. At `wj-ia-300` that is cap 91.
  `gate.json` records the cap as `grip_cap`.
- One pane: G1 runs `view.local`, the Local hide. Local's opening closes and Global draws alone. The row shows Local
  again on its cap, with the camera still shared. In a live session the key L stays the router's twist key, so the row
  runs from the palette or from a script.
- Window: the display's native size, read when the run starts (`DESKTOPHORZRES` x `DESKTOPVERTRES`, 2560x1600 on the
  owner's laptop). `--resolution WxH` overrides it. `gate.json` records the size and its source.
- Foreground: by default the window never takes the foreground. `--foreground` is for the sitting with the owner
  present only: the window takes the foreground once at the start (one `SetForegroundWindow`, never topmost), and the
  checks of the owner's window rule then ask only that it is never topmost. `gate.json` records which. Start such a run
  from a PowerShell in front on the desktop, or Windows refuses the foreground and the run is invalid.
- Occlusion: a click step's fractions are of Global's opening as the shell lays it out at run time, so the grid
  (default: all of the opening) follows Global's pane. The record holds the opening the dumps report and the number of
  click steps that landed in it; a click outside it makes the run invalid. The dumps are the record of the clicks (the
  shell writes no click row to `events.jsonl`): a click step counts as inside when its dump reports the opening, its
  fractions lie in [0, 1], it asked exactly one pick and any hit names Global. `--occlusion --rescore <folder>` scores
  a run folder again from its dumps into `gate-rescored.json` and keeps its first `gate.json`.

How to run a sitting:

1. Write the fixtures first: `python -B renderer/shell/build_d02_fixtures.py --state deep-2`, then `--state wj-ia-end`
   and `--state wj-ia-300` (sitting 1 needs all three; sitting 2 needs `deep-2`). The writer is Rust.
2. Build the product once: `python -B renderer/shell/build_product.py`.
3. Run each row from a PowerShell on the desktop:
   `python -B renderer/shell/gate_d02.py --row G1 --foreground` (drop `--foreground` if the owner chose the background
   window). Allow for the reopen: a deep state (`deep-2`) takes about four minutes before the row's views are set, a
   `wj-ia` state well under a minute. A row then runs 10 s of warm-up and 60 s of window.
4. After every row, read the System log for nvlddmkm (ids 14 and 153) and LiveKernelEvent since the row started. At the
   first such event, stop the sitting.

Dry runs (not gate evidence, never recorded as results), 27 November 2026, at `wj-ia-300`, 10 s window, background
window at 2560x1600, cap 91: G1 drew one pane in all 435 window frames and committed 7 twists; G9 drew two panes in all
428 and committed 7. Both runs were valid, and the System log showed no GPU event after either.

## Sitting 1 (B19, about 35 minutes with the owner at the start)

| Row | State | Views and script | Command | Panes | Result | Evidence |
|---|---|---|---|---|---|---|
| G1 | `deep-2` | Local hidden, Global with the contact-class tint, twist sweeps repeating (the one-view baseline) | `python -B renderer/shell/gate_d02.py --row G1 --out work/gate-d02/g1` | 1 | pass: `D02_GATE row=G1 panes=1 view_vertex_evals=18288000 fps_avg=59.65 p99_ms=16.67 vram_gb=0.27 pass=true` (cap 2) | `work/gate-d02/g1-20261127T194828/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G9 | `deep-2` | G1's script with Local shown | `python -B renderer/shell/gate_d02.py --row G9 --out work/gate-d02/g9` | 2 | pass: `D02_GATE row=G9 panes=2 view_vertex_evals=36576000 fps_avg=59.68 p99_ms=16.67 vram_gb=0.29 pass=true` (cap 2) | `work/gate-d02/g9-20261127T195955/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G2 | `wj-ia-end` | two views, a blocked cap named (cap 0), all line layers | `python -B renderer/shell/gate_d02.py --row G2 --out work/gate-d02/g2` | 2 | pass: `D02_GATE row=G2 panes=2 view_vertex_evals=36576000 fps_avg=60.00 p99_ms=16.67 vram_gb=0.29 pass=true` | `work/gate-d02/g2-20261127T201114/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G3 | `wj-ia-300` (fallback, below) | two views, the blocked grip with the most recorded certificate points | `python -B renderer/shell/gate_d02.py --row G3 --out work/gate-d02/g3` | 2 | pass: `D02_GATE row=G3 panes=2 view_vertex_evals=36576000 fps_avg=60.00 p99_ms=16.67 vram_gb=0.29 pass=true`; certificate points: 462 beside 1,654 | `work/gate-d02/g3-20261127T201350/gate.json`, build `c9-product-1a6a967326354a35`, valid |

## Sitting 2 (B24, about 45 minutes with the owner at the start)

| Row | State | Views and script | Command | Panes | Result | Evidence |
|---|---|---|---|---|---|---|
| G4 | `deep-2` | Global chain, Local slice, twists of the named cap | `python -B renderer/shell/gate_d02.py --row G4 --out work/gate-d02/g4` | 2 | pass: `D02_GATE row=G4 panes=2 view_vertex_evals=36576000 fps_avg=59.67 p99_ms=16.67 vram_gb=0.29 pass=true` (cap 2) | `work/gate-d02/g4-20261127T201528/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G5 | `deep-2` | Global chain, Local raw | `python -B renderer/shell/gate_d02.py --row G5 --out work/gate-d02/g5` | 2 | pass: `D02_GATE row=G5 panes=2 view_vertex_evals=36576000 fps_avg=60.00 p99_ms=16.67 vram_gb=0.29 pass=true` | `work/gate-d02/g5-20261127T202651/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G6 | `deep-2` | Global ring sheet, Local chain carrying the sweeps | `python -B renderer/shell/gate_d02.py --row G6 --out work/gate-d02/g6` | 2 | pass: `D02_GATE row=G6 panes=2 view_vertex_evals=36576000 fps_avg=59.62 p99_ms=16.67 vram_gb=0.29 pass=true` (cap 2) | `work/gate-d02/g6-20261127T203756/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G7 | `deep-2` | Global Hopf rings, Local chain | `python -B renderer/shell/gate_d02.py --row G7 --out work/gate-d02/g7` | 2 | pass: `D02_GATE row=G7 panes=2 view_vertex_evals=36576000 fps_avg=60.00 p99_ms=16.67 vram_gb=0.29 pass=true` | `work/gate-d02/g7-20261127T204910/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G10 | `deep-2` | Global chain, Local slice with the split held | `python -B renderer/shell/gate_d02.py --row G10 --out work/gate-d02/g10` | 3 | pass: `D02_GATE row=G10 panes=3 view_vertex_evals=54864000 fps_avg=60.00 p99_ms=16.67 vram_gb=0.29 pass=true` | `work/gate-d02/g10-20261127T210014/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G11 | `deep-2` | Global ring sheet, Local raw with the antipode (the heaviest case) | `python -B renderer/shell/gate_d02.py --row G11 --out work/gate-d02/g11` | 3 | pass: `D02_GATE row=G11 panes=3 view_vertex_evals=54864000 fps_avg=60.00 p99_ms=16.67 vram_gb=0.29 pass=true` | `work/gate-d02/g11-20261127T211117/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| G8 | `deep-2` | the simulation matrix held (B14's view style) on G9's two views | `python -B renderer/shell/gate_d02.py --row G8 --out work/gate-d02/g8` | 2 | pass: `D02_GATE row=G8 panes=2 view_vertex_evals=36576000 fps_avg=59.67 p99_ms=16.67 vram_gb=0.31 pass=true` (cap 2) | `work/gate-d02/g8-20261127T212222/gate.json`, build `c9-product-1a6a967326354a35`, valid |
| Occlusion | `deep-2` | ring sheet and Hopf rings at the default camera, a 24 x 14 grid of picks over Global | `python -B renderer/shell/gate_d02.py --occlusion --out work/gate-d02/occlusion` | - | valid after rescoring: `D02_OCCLUSION preset=ring_sheet grid=24x14 clicks=336 hits=23 distinct_pieces=23 pieces=177120 seen_fraction=0.0001`; `D02_OCCLUSION preset=hopf_rings grid=24x14 clicks=336 hits=43 distinct_pieces=43 pieces=177120 seen_fraction=0.0002`; 672 of 672 click steps inside Global's opening [327, 110, 946, 1093], which never moved | `work/gate-d02/occlusion-20261127T213337/gate-rescored.json`, build `c9-product-1a6a967326354a35`; the first `gate.json` (kept) said invalid (exit 3, "0 of 0 clicks (of 672) landed in Global's opening"): its check read click rows from `events.jsonl`, which the shell never writes |

## The sitting of 27 November 2026

- Build `c9-product-1a6a967326354a35`, built from `claude/adr-d02-build` at 8b0b746d. The fixtures came from step 1,
  and every row ran at its own state.
- The owner's RTX 4070 Laptop GPU, at the native 2560x1600 window. The owner turned the GPU overclock off at about 19:47
  UTC, before the first row.
- Every row ran with `--foreground`, from a PowerShell in front, and every record holds `"foreground": true`. The
  window was never topmost. As in every `--foreground` run, the window rule's dump reads `owner_acted` once the shell
  holds the foreground and `d02-run.json` reads `foreground_kept: false`. Neither invalidates a run.
- The rows ran back to back, with the run folders' start times in UTC: G1 19:48, G9 19:59, G2 20:11, G3 20:13 (sitting
  1); G4 20:15, G5 20:26, G6 20:37, G7 20:49, G10 21:00, G11 21:11, G8 21:22, occlusion 21:33 to 21:50 (sitting 2).
  Each `deep-2` row took about 11 minutes, mostly the reopen.
- The System log showed no nvlddmkm event (ids 14 and 153) and no LiveKernelEvent from 19:45 UTC to the end of the
  sitting.
- Every row's p99 frame time is 16.67 ms and every average is 59.6 to 60.0 fps: the presentations keep one 60 Hz
  refresh interval. The limits (30 fps, 33.3 ms) are met with room, and the peak VRAM is 0.27 to 0.31 GB of the 7.0 GiB
  limit.
- The three-pane rows (G10, G11) pass, so neither the split nor the antipode is withheld. G9 passes.

## Evidence kind

- A result counts as gate evidence only from a run on the owner's RTX 4070 Laptop GPU, on the build named in its
  `gate.json`, at the row's own state. That is actual Windows/DirectX evidence for that build only.
- A run with `--state` overridden or a shorter `--seconds` is a dry run. Its numbers are never recorded here.
- The occlusion line is a sampled count: each grid point is one pick pass, so a piece that no grid point reaches may
  still show between the points.

## How a result is recorded

- The row's cell takes the `D02_GATE` line as printed, the run folder's name and the build identity. A failing row is
  recorded as it is and is never tuned away: no setting, state or script changes to make it pass.
- A failing G9 goes to the owner, as does any failing three-pane row (G10, G11). A failing three-pane row withholds
  only the split or the antipode.
- An invalid run (exit 3) is not a result. Its reason is written beside the row, and the row runs again once the cause
  is fixed. When the cause was the checker alone and the run's own records are complete (run code 0, no mismatch, its
  dumps and event log written), the fixed checker scores the same run again instead (`--rescore`); the first record is
  kept and the row says so.

## What the sittings need first

- **G3's state.** S1 of `research/jumbling/witness.py` cannot be reached. Its twist (c, g) of 9.99987° (s = 15/961, g not
  in A4(c)) is no letter of the pinned I_a menu, and the fixture writer (`c600-shell-session-fixture --word`) commits
  only menu letters. Section 6's fallback applies: the sample state whose blocked grip has the most certificate points.
  - Among the written cap records that is `wj-ia-300`'s cap 0 with 462 points. The other records are `wj-ia-end` with
    390 and `deep-2` with 376.
  - Finding the true maximum over every blocked grip of every state needs the Rust adapter's straddler survey
    (`tools/looklab/fixtures/make_d02_samples.py`, which writes cap records through `RustJumbleEngine`, with
    `pick_caps` widened to the blocked grips).
- **Foreground.** The product window never takes the foreground by default (owner, 26 November 2026). The dry runs
  presented with the window in the background. The owner chose the foreground for the sitting (27 November 2026: "占前台，
  明天我不在家，最好今天晚上能跑完", take the foreground; I am away tomorrow, so it is best to finish tonight), so every
  row ran with `--foreground`.
- **Fixtures.** Write each state's session fixture first (step 1 above). A deep fixture replays about 3,300 commits,
  and a deep state takes about four minutes to reopen in every row.
