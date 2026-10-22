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
| 7. Records, trees and the independent check | in revision |

## How to read it

- Every quantity is a function of the current state. Each chapter states the commit its `path:line` citations refer
  to.
- A value marked **computed** comes from the chapter's own script, `scripts/chNN_examples.py`, which reads only
  committed files and writes `records/chNN-examples.json`. The scripts run on development seeds only; the seeds of
  the acceptance batches (9011 to 9073) appear nowhere.
- Evidence kind: source and fixtures, run headless on Linux. Nothing here is Windows, Direct3D or performance
  evidence.

## Figures

- **Puzzle pictures** (`figures/png/`) show exact simulator states: all 259,800 stickers of the state, the pieces a
  step moves drawn opaque. They are headless Chromium captures of the research page
  [`research/jumbling/viewer/full.html`](../viewer/full.html) in its shot mode, made by
  [`make_figures.py`](../viewer/make_figures.py) from [`figures/figures.json`](figures/figures.json); each has a
  manifest entry with its state digest and the PNG's sha256, and `--check` draws it again and compares. They are not
  pictures of the Direct3D 12 renderer.
- **Diagrams** (`figures/svg/`) are drawn only where nothing geometric is shown: step trees, flows, rule tables,
  graphs and charts.
- Colours are provisional and decide nothing for the design track; G1 to G6 stay with the owner.
- [`figures/CATALOGUE.md`](figures/CATALOGUE.md) lists every figure with its source and status; a figure that cannot
  yet be drawn is listed as missing with the reason.

## Reproduce

```
python3 -I -B research/jumbling/tutorial/scripts/ch01_examples.py --out <new file> --cache <new directory>
python3 -I -B research/jumbling/viewer/make_figures.py --list research/jumbling/tutorial/figures/figures.json \
    --out research/jumbling/tutorial/figures --vendor <directory with three.js 0.160.0> --check
```

The other chapter scripts take the options their headers name.
