# Jumbling theory: documents, scripts and results

Index of `research/jumbling/theory/`. Every script runs from the repository root on CPython 3 with NumPy, on top of the J1 reference (`research/jumbling/sim/`). Evidence kind: source and exact synthetic geometry. Timings are cloud timings of research code, not performance evidence. Open problems are listed in [open-problems.md](open-problems.md).

## Documents

| File | What it is |
| --- | --- |
| [theory-draft.md](theory-draft.md) | J3 theory draft: groupoid and validity (item 1), blocking (2), finiteness (3), lattice states (4), defect and return (5), solving theory (6), scope (7). |
| [restoration.md](restoration.md) | J3-R restoration theory and method: tree model, misalignment N and the rim test, phase U (rim descent and the deep-input rules), phase L, and what is proved, computed, a lead or open. |
| [open-problems.md](open-problems.md) | Every open jumbling problem with its status, latest evidence and next step. |
| [cards/](cards/README.md) | Self-contained problem cards for an external prover (L1, L2, R1). |

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
| `e1_carrier.py` | Joint-attack E1: return a whole carrier first (one twist, taken even if N rises); retained-only words rejected. | `python research/jumbling/theory/e1_carrier.py I_a 40 <out.json> [--start <moves.json> --prefix k]` | open-problems.md, R1 joint attack |
| `e2_frag.py` | Joint-attack E2: no solver record, potential (off-lattice count, N) in lexicographic order, lens commutator rule. | `python research/jumbling/theory/e2_frag.py I_a 40 research/jumbling/theory/results/seams-I_a-40-stop.json <out.json> --order seam,cover,close,carrier,comm,lock,conj,improve,shift` | open-problems.md, R1 joint attack |
| `isolate.py` | Each off-lattice domain: the caps that hold it whole and the smallest intersection of 1 to 4 of them. | `python research/jumbling/theory/isolate.py I_a 40 research/jumbling/theory/results/seams-I_a-40-stop.json` | open-problems.md, R1 joint attack |
| `comm_anatomy.py` | One commutator piece by piece: which of its four twists move each piece, and which pieces leave or reach the lattice. | `python research/jumbling/theory/comm_anatomy.py I_a 40 research/jumbling/theory/results/seams-I_a-40-stop.json 30 44 145 39` | open-problems.md, R1 joint attack |
| `deep_scramble.py` | The five inscribed 600-cells of the grip poles, their blocking, a scramble with most pieces off the lattice, and an exact check of the lattice-prefix rule. | `python research/jumbling/theory/deep_scramble.py I_a` | open-problems.md, structure results |
| `classes.py` | Under I_a, exactly: the five classes of grip poles (n_c·2I) and K⁺ acting on them, the menu as a stabiliser in the rotation group of its class, compatible poles, the 20 frames and the cells of each class's cuts; with `--states`, the class of every off-lattice domain and the cells its fragments occupy at the deep scramble, the I_a@40 stop and the E2 end. | `python research/jumbling/theory/classes.py --states` | open-problems.md, structure results; card 3 |
| `e3_class.py` | E3: the class-1 cell model at the E2 end of I_a@40 (contents, labels, free grips, the star of each cap, orbits of (cell, label) points), the search for a word of class-1 twists that returns F50 (through the hole) and F48 (a holonomy cycle of edge twists), the model check and the exact replay. | `python research/jumbling/theory/e3_class.py` (`--verify` replays the recorded word) | open-problems.md, structure results (E3) |
| `mixed_cells.py` | The four fragments at the E2 end of I_a@40 that share a cell with lattice pieces: shared cells with their vacant chambers, pieces inside the outline, the other classes' cuts that separate a shared cell and what their retained twists do, the menu elements of each grip that preserve another class, and the class-1 cell types (label-0 orbit size, chambers). | `python research/jumbling/theory/mixed_cells.py` | cards/card-4 (M1 to M3, G1 to G3) |
| `orient.py` | The orientation model of one class: every content of the class puzzle with its accumulated element of Sym⁺(P) (a pair of units in standard coordinates), the labelled permutation of a word of free twists, the predicted pose and chamber of every piece of the lattice and of the class's frames, and the exact check that replays words on the simulator and compares every piece. Starts at the E2 end, at a current-only pose table or after word files. | `python research/jumbling/theory/orient.py --cls 0 1 2 3 4 --check 8 --retained-checks 4 --out <file>` (`--words research/jumbling/theory/results/e-chain/*.json` for the 277 state) | open-problems.md (R1) |
| `split_pairs.py` | Glued pairs at a state (an off-lattice piece on the same side of all 600 cuts as a lattice piece, which no retained word separates), the jumble steps that separate each pair with the separating cuts admissible after them, and the loops J C J⁻¹ C⁻¹ with their legality and collateral, each applied and undone on the exact simulator. | `python research/jumbling/theory/split_pairs.py --words research/jumbling/theory/results/e-chain/*.json --loops 1000 --out <file> [--full <file>]` (`e-chain/[1-4]-*.json` for the 388 state) | open-problems.md (R1) |

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
| `isolate-I_a-40-stop.txt`, `comm-anatomy-I_a-40-stop.txt` | `isolate.py`, `comm_anatomy.py` | Fragment isolation and the anatomy of the first lowering commutator at the I_a@40 stop | open-problems.md, R1 joint attack |
| `joint-attack-I_a-40.json` | `e1_carrier.py`, `e2_frag.py`, a scratch enumeration | Floor and complete three-twist enumeration on two cap pairs; E1 from S31; step logs of E1 on S4@40 and E2 at the stop (partial when marked) | open-problems.md, R1 joint attack |
| `deep-scramble-I_a.json` | `deep_scramble.py` | Inscribed 600-cells and their coverage, blocking profile and set-neighbour blocking, the 360-twist scramble (moves, 93.5% off the lattice), lattice-prefix check | open-problems.md, structure results |
| `classes-I_a.json` | `classes.py --states` | Class structure checks; fragments by class, cells and blocked grips at three configurations | open-problems.md, structure results; card 3 |
| `e3-class1-I_a-40.json` | `e3_class.py` | Cell model and orbit structure of class 1 at the E2 end, the E3 search records, the 312-twist word and its exact replay | open-problems.md, structure results (E3) |
| `mixed-cells-I_a-40.json` | `mixed_cells.py` | Shared cells, vacant chambers, separating cuts, retained-twist trials, menu intersections and class-1 cell types at the E2 end | cards/card-4 |
| `orient-checks-e2.json` | `orient.py --cls 0 1 2 3 4 --check 8 --retained-checks 4 --length 25 --check-words research/jumbling/theory/results/e3-class1-I_a-40.json` | The model of each class at the E2 end (free and blocked grips, fragments with their labels) and its checks on the simulator: random words of all and of retained generators, and the E3 word | open-problems.md (R1) |
| `orient-checks-388.json`, `orient-checks-277.json` | `orient.py --words research/jumbling/theory/results/e-chain/[1-4]-*.json` (388) or `e-chain/*.json` (277) `--cls 0 1 2 3 4 --check 8 --retained-checks 4 --length 25` | The same at the 388 and 277 states | open-problems.md (R1) |
| `split-pairs-388.json`, `split-pairs-277.json` | `split_pairs.py --words research/jumbling/theory/results/e-chain/[1-4]-*.json` (388) or `e-chain/*.json` (277) `--loops 1000` | The one glued pair at the 388 and 277 states, its separating jumble steps and every loop through an admissible separating cut (word, end digest, collateral counts; piece lists for the 20 loops with the fewest other pieces moved) | open-problems.md (R1) |
| `e-chain/` | handed over by the owner from the Codex and Rethlas runs (11 October 2026) | The words from the E2 end to 547 off-lattice pieces (6,917 twists), to 499 (238), to the prepared 499 state (2,473), to 388 (82), to the prepared 388 state (4,184) and to 277 (88), each with its input and output digest (`sim.State.digest()`); replayed here from the E2 end with every twist checked admissible and every digest matched | open-problems.md (R1) |
