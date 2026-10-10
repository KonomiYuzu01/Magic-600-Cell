# Jumbling theory: documents, scripts and results

Index of `research/jumbling/theory/`. Every script runs from the repository root on CPython 3 with NumPy, on top of the J1 reference (`research/jumbling/sim/`). Evidence kind: source and exact synthetic geometry. Timings are cloud timings of research code, not performance evidence. Open problems are listed in [open-problems.md](open-problems.md).

## Documents

| File | What it is |
| --- | --- |
| [theory-draft.md](theory-draft.md) | J3 theory draft: groupoid and validity (item 1), blocking (2), finiteness (3), lattice states (4), defect and return (5), solving theory (6), scope (7). |
| [restoration.md](restoration.md) | J3-R restoration theory and method: tree model, misalignment N and the rim test, phase U (rim descent and the deep-input rules), phase L, and what is proved, computed, a lead or open. |
| [open-problems.md](open-problems.md) | Every open jumbling problem with its status, latest evidence and next step. |

## Scripts

| Script | What it computes | Command | Used in |
| --- | --- | --- | --- |
| `groups.py` | Exact S4₀ and the icosahedral groups I_a, I_b containing A4₀ (cap frame and lifted). | imported by the others | theory draft |
| `blocking.py` | Blocked grips after one twist from solved, predicted from the cap polytope, against the J1 survey. | `python research/jumbling/theory/blocking.py <out.json>` | theory draft item 2 |
| `invariants.py` | Fixed centre points (Proposition 4.1) and the infinite-order certificate of the E2 witness. | `python research/jumbling/theory/invariants.py <out.json> [--replay]` | theory draft items 3, 4 |
| `walk.py` | Exact random walks: pose table, tree heights, blocked counts (leads). | `python research/jumbling/theory/walk.py <menu> <steps> <seed> <seconds> <out.json>` | theory draft items 3, 5 |
| `conj.py`, `conj2.py` | The words (0, q)(d, a)(0, q⁻¹): admissibility, lattice endpoints, whether an endpoint is a single retained twist. | `python research/jumbling/theory/conj.py <menu> <out.json>`, then `conj2.py <results dir>` | theory draft item 4 |
| `trees.py` | Left and right quaternion factors of every pose on the two Bruhat–Tits trees at 2; `--check` verifies the tree model, `--fixtures` gives height histograms. | `python research/jumbling/theory/trees.py --check` / `--fixtures` | restoration section 1 |
| `rim.py` | Exact misalignment N_c of a cut and the total N. | `python research/jumbling/theory/rim.py --demo` | restoration section 2 |
| `restore.py` | Phase U rules close, lock, improve and shift (U2 to U5′) on fixtures, random scrambles and lens words. | `python research/jumbling/theory/restore.py random I_a 10 1` (see its docstring) | restoration sections 3, 5 |
| `lens.py` | The abelian invariant v on the retained generators and the lens words. | `python research/jumbling/theory/lens.py` | restoration section 4 |
| `seams.py` | Deep-input rules: face match, cover order, conjugate repair, reopening (U1a, U1b, U5″, U7); `facecheck` counts; `batch` runs jobs in parallel. | `python research/jumbling/theory/seams.py fixture I_a 40` (see its docstring) | restoration sections 3, 5 |
| `paths.py` | Diagnosis only (reads the journal): N, tree height, off-lattice pieces and domains along the true scramble path, and the shortest reverse prefix that lowers each. | `python research/jumbling/theory/paths.py S4 60` | deferred-problems record (bound on lowering words) |

## Results (`results/`)

| Files | Written by | Content | Used in |
| --- | --- | --- | --- |
| `blocking.json` | `blocking.py` | Predicted against surveyed blocked grips | theory draft item 2 |
| `invariants.json`, `invariants.log` | `invariants.py` | Centre points and the infinite-order certificate | theory draft items 3, 4 |
| `walk-<menu>-seed<n>.json` | `walk.py` | Random-walk statistics, three seeds per menu | theory draft items 3, 5 |
| `conj-S4.json`, `conj-I_a.json`, `conj2.json` (+ `.log`) | `conj.py`, `conj2.py` | Lens-word admissibility and endpoints | theory draft item 4 |
| `trees.json`, `trees-fixtures.json` | `trees.py` | Tree-model checks; height histograms along the W-J fixtures | restoration section 1 |
| `restore-fixture-<menu>.json` | `restore.py` | Phase U on the W-J fixtures (20 records; S4@60 stops at N = 7,472) | restoration sections 3, 5 |
| `restore-random-<menu>.json` | `restore.py` | Phase U on random scrambles (k = 10, 20, mixed) | restoration section 5 table |
| `restore-lens-<menu>.json` | `restore.py` | Phase U on the lens words | restoration section 5 table |
| `lens-invariants.json` | `lens.py` | v on the generators and the lens configurations | restoration section 4 |
| `seams-facecheck-<menu>.json` | `seams.py facecheck` | Face match against top records at X_t | restoration section 3 (U1a table) |
| `seams-random-S4-10-1.json` | `seams.py random S4 10 1` | Smoke run: lattice, solved | review checks |
| `seams-S4-60-stop.json` | earlier cover-order run | S4@60 stopping configuration (59 twists, N = 6,294); usable as `--start` | restoration section 3 |
| `seams-I_a-40-stop.json` | `seams.py` (default order) | I_a@40 stopping configuration (62 twists, N = 1,192); usable as `--start` | open-problems.md, R1 |
| `seams-fixture-<menu>-<t>-partial.json` | `seams.py` | Step logs of long runs, stopped or still running when saved; not end results | restoration section 3 table |
| `paths-<menu>-<t>.json` | `paths.py` | True-path statistics (S4 to 60 and 640 records, I_a and I_b to 300) | deferred-problems record |
