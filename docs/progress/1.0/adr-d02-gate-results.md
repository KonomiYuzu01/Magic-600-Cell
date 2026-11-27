# ADR-D02 gate results (B19 and B24)

The gate rows of the ADR-D02 build (`docs/progress/1.0/adr-d02-build-framework.md`, B19 "Gate sitting 1" and B24 "Gate
sitting 2"). Each row is one command. The limits are the renderer selection gate's (`docs/wiki/decisions/renderer-candidates.md`):
an average of at least 30 fps, a 99th-percentile frame time of at most 33.3 ms and a peak VRAM of at most about 7 GB
(read as 7.0 GiB), at native resolution with no frame generation.

**Status: tools built, sittings not run.** No cell below holds a result yet.

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

## Sitting 1 (B19, about 35 minutes with the owner at the start)

| Row | State | Views and script | Command | Panes | Result | Evidence |
|---|---|---|---|---|---|---|
| G1 | `deep-2` | Local hidden, Global with the contact-class tint, twist sweeps repeating (the one-view baseline) | `python -B renderer/shell/gate_d02.py --row G1 --out work/gate-d02/g1` | 1 | not run (needs the owner's sitting) | none yet |
| G9 | `deep-2` | G1's script with Local shown | `python -B renderer/shell/gate_d02.py --row G9 --out work/gate-d02/g9` | 2 | not run (needs the owner's sitting) | none yet |
| G2 | `wj-ia-end` | two views, a blocked cap named (cap 0), all line layers | `python -B renderer/shell/gate_d02.py --row G2 --out work/gate-d02/g2` | 2 | not run (needs the owner's sitting) | none yet |
| G3 | `wj-ia-300` (fallback, below) | two views, the blocked grip with the most recorded certificate points | `python -B renderer/shell/gate_d02.py --row G3 --out work/gate-d02/g3` | 2 | not run (needs the owner's sitting); certificate points: 462 beside 1,654 | none yet |

## Sitting 2 (B24, about 45 minutes with the owner at the start)

| Row | State | Views and script | Command | Panes | Result | Evidence |
|---|---|---|---|---|---|---|
| G4 | `deep-2` | Global chain, Local slice, twists of the named cap | `python -B renderer/shell/gate_d02.py --row G4 --out work/gate-d02/g4` | 2 | not run (needs the owner's sitting) | none yet |
| G5 | `deep-2` | Global chain, Local raw | `python -B renderer/shell/gate_d02.py --row G5 --out work/gate-d02/g5` | 2 | not run (needs the owner's sitting) | none yet |
| G6 | `deep-2` | Global ring sheet, Local chain carrying the sweeps | `python -B renderer/shell/gate_d02.py --row G6 --out work/gate-d02/g6` | 2 | not run (needs the owner's sitting) | none yet |
| G7 | `deep-2` | Global Hopf rings, Local chain | `python -B renderer/shell/gate_d02.py --row G7 --out work/gate-d02/g7` | 2 | not run (needs the owner's sitting) | none yet |
| G10 | `deep-2` | Global chain, Local slice with the split held | `python -B renderer/shell/gate_d02.py --row G10 --out work/gate-d02/g10` | 3 | not run (needs the owner's sitting) | none yet |
| G11 | `deep-2` | Global ring sheet, Local raw with the antipode (the heaviest case) | `python -B renderer/shell/gate_d02.py --row G11 --out work/gate-d02/g11` | 3 | not run (needs the owner's sitting) | none yet |
| G8 | `deep-2` | the simulation matrix held (B14's view style) on G9's two views | `python -B renderer/shell/gate_d02.py --row G8 --out work/gate-d02/g8` | 2 | not run (needs the owner's sitting) | none yet |
| Occlusion | `deep-2` | ring sheet and Hopf rings at the default camera, a 24 x 14 grid of picks over Global | `python -B renderer/shell/gate_d02.py --occlusion --out work/gate-d02/occlusion` | - | not run (needs the owner's sitting) | none yet |

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
  is fixed.

## What the sittings need first

- **G3's state.** S1 of `research/jumbling/witness.py` cannot be reached. Its twist (c, g) of 9.99987° (s = 15/961, g not
  in A4(c)) is no letter of the pinned I_a menu, and the fixture writer (`c600-shell-session-fixture --word`) commits
  only menu letters. Section 6's fallback applies: the sample state whose blocked grip has the most certificate points.
  - Among the written cap records that is `wj-ia-300`'s cap 0 with 462 points. The other records are `wj-ia-end` with
    390 and `deep-2` with 376.
  - Finding the true maximum over every blocked grip of every state needs the Rust adapter's straddler survey
    (`tools/looklab/fixtures/make_d02_samples.py`, which writes cap records through `RustJumbleEngine`, with
    `pick_caps` widened to the blocked grips).
- **Twist sweeps.** The router's twist keys turn the constant grip cap 0 (`ShellTurn.GripCap`, while `grip.select` is not
  built). Cap 0 is blocked at `deep-2`, `wj-ia-end` and `wj-ia-300`, so the sweeping rows (G1, G9, G4, G6, G8) are
  refused there. Those rows need `grip.select`, or a twist at the named cap, before they can run (row request).
- **G1's hidden Local.** Local is hidden only while no state is bound. The keymap's Local row (`view.local`, key L in
  Global) has no handler in the shell, so G1 runs with two panes, which is invalid for G1 until it does.
- **Foreground.** The product window never takes the foreground (owner, 26 November 2026). The dry run presented at a
  median of 33.3 ms per frame with the window in the background. Whether the sitting's window may hold the foreground,
  as earlier measurement runs did, is the owner's call before sitting 1.
- **Fixtures.** Write each state's session fixture first with `python -B renderer/shell/build_d02_fixtures.py --state deep-2`
  (and `wj-ia-end`, `wj-ia-300`). The writer is Rust, and a deep fixture replays about 3,300 commits. A deep state takes
  about four minutes to reopen in every row.
