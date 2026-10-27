# The jumbling tutorial

How the tutorial runner restores the jumbled puzzle, rule by rule, so that a reader can follow every step of a
record and check it. The runner is
[`research/jumbling/restoration/13-tutorial-runner/scripts/runner.py`](../restoration/13-tutorial-runner/scripts/runner.py);
its acceptance test is the owner's (decisions of
[18 October](../../../docs/wiki/decisions/owner-decisions-2026-10-18-machine-tutorial.md),
[20 October](../../../docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md) and
22 October 2026: [tutorial first](../../../docs/wiki/decisions/owner-decisions-2026-10-22-tutorial-first.md),
[figures and merge](../../../docs/wiki/decisions/owner-decisions-2026-10-22-figures-and-merge.md)).

## Chapters

| Chapter | Subject |
|---|---|
| [1. What the machine reads](01-state.md) | caps, the menu I_a, pieces and poses, letters and words, domains, heights, misalignment, seams, units, the digest, snapshots, entry tests, step trees |
| [2. The bulk descent](02-bulk.md) | seam, cover, close, conj, lock, improve and shift; the decreasing-N contract; why the bulk ends |
| [3. Height](03-height.md) | r22d75 (T0, T1, T2), r22d81 with its pairs, h4 with pair steps; termination |
| [4. High](04-high.md) | the kernel: F1, F2, F3, D19, S·T and T1 in blocker order; gates and the letter bound |
| [5. The tail](05-tail.md) | units, zero_bad, pure, close (V·W·V⁻¹), t1prime (W·V′·W⁻¹·V′⁻¹), the fallback and the probe |
| [6. The lattice phase](06-lattice.md) | corrections, the cascade order, orbit steps under schedule11, the literal player, identity labels |
| [7. Records, trees and the independent check](07-records.md) | the run directory and `run.json`, step rows, words, step trees, phase results, the acceptance verdict, the two modes, clocks, the independent checker |
| [Appendix A. Macros](A-macros.md) | every step with named parts: its shape, rule, step tree and the region it acts on |

## How to read it

- Every quantity is a function of the current state. Each chapter states the commit its `path:line` citations refer
  to.
- A value marked **computed** comes from the chapter's own script, `scripts/chNN_examples.py`, which reads only
  committed files and writes `records/chNN-examples.json`. The scripts run on development seeds only; the seeds of
  the acceptance batches (9011 to 9073) appear nowhere.
- Evidence kind: source and fixtures, run headless on Linux. Nothing here is Windows, Direct3D or performance
  evidence.

## Figures

- **Puzzle pictures** (`figures/png/`) show exact simulator states. Each draws the pieces its recipe emphasises
  opaque (the pieces a step moves, the off pieces, or a named set); a few overview pictures also draw every other
  sticker as a faint ghost. Nothing else is drawn on the puzzle: no outlines, numbers or cuts; panel labels sit below
  the panels. A picture is framed on all its emphasised pieces, on a cap and its four nearest cells (`fit` `cap`), or on
  the densest three quarters of the emphasised pieces (`fit` `core`); the manifest records how many emphasised pieces
  lie in the frame. They are headless Chromium captures of the research page
  [`research/jumbling/viewer/full.html`](../viewer/full.html) in its shot mode, made by
  [`make_figures.py`](../viewer/make_figures.py) from [`figures/figures.json`](figures/figures.json); each has a
  manifest entry with its state digest and the PNG's sha256, and `--check` draws it again and compares. They are not
  pictures of the Direct3D 12 renderer.
- **Diagrams** (`figures/svg/`) are drawn only where nothing geometric is shown: step trees, flows, rule tables,
  graphs and charts.
- Colours are provisional and decide nothing for the design track; G1 to G6 stay with the owner.
- [`figures/CATALOGUE.md`](figures/CATALOGUE.md) lists every figure with its source and status; a figure that cannot
  yet be drawn is listed as missing with the reason.

## Development runs

The runner's development runs (stage 13 `PLAN.md` section 4; records under
`research/jumbling/restoration/13-tutorial-runner/runs/dev/`) are **run on one instance** each, headless on Linux.

| Run | Input | Result | Chapters |
|---|---|---|---|
| D0 | seeds 7001 (depth 10) and 7106 (depth 20), acceptance mode | both restored to the solved state; the checker passes both records | 2.15, 6.17.1 |
| D1 | seed 7101 (depth 40), development mode | the bulk stuck at 1,193 off; one tail step (`zero_bad`, 1,193 → 1,082 off); killed by a container restart in the tail, not checked | 2.15, 5.18 |
| D2 | the 3000-twist census stops of seeds 7302 and 7304 to 7307 with the bulk off, and seed 7303 from its scramble | no run reached the lattice phase: the height phase stuck with n4 between 734 and 1,955; the high phase stuck (`kernel_exhausted`) at 7304 to 7307, ran out of its clock at 7303 and was cut off by a lost container at 7302; the tail stopped while listing the units at 7304, 7305 and 7307 and was cut off by a lost container at 7306; four of the six records checked | 2.15, 3.8.4, 4.18, 5.18 |

The acceptance-mode run at depth 40 (seed 7101, run 4) stopped `stuck` in the tail with 158 pieces off, all at
height 2, and its check passes; card 9 completed that stop outside the runner
([restoration stage 18](../restoration/18-tail-completion/README.md)). The run at depth 100 (D3, seed 7102) was
stopped at the freeze at its last checkpoint, 530 bulk steps with 14,528 pieces off, and was not checked
([its record](../restoration/13-tutorial-runner/runs/dev/d3-s7102/RESULT.md)).

**The tutorial is frozen and shelved, unaccepted** (owner, 26 and 27 October 2026:
[decision](../../../docs/wiki/decisions/owner-decisions-2026-10-27-freeze.md)). None of the 21 acceptance scrambles
has run (seeds 9011 to 9073): batch 1 (depths 10, 20, 40 and 100) waited for the development runs at those depths to
reach the solved state (owner, 22 October 2026), and the batches at 300, 1000 and 3000 twists for rules that take
height 4 to height 2 ([decision](../../../docs/wiki/decisions/owner-decisions-2026-10-23-depth.md)).

## Reproduce

```
python3 -I -B research/jumbling/tutorial/scripts/ch01_examples.py --out <new file> --cache <new directory>
python3 -I -B research/jumbling/viewer/make_figures.py --list research/jumbling/tutorial/figures/figures.json \
    --out research/jumbling/tutorial/figures --vendor <directory with three.js 0.160.0> --check
```

The other chapter scripts take the options their headers name.
