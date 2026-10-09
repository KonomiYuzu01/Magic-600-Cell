# Four-dimensional grip-orbit explorer (J4)

Status: **research code for workstream J4 of the [jumbling plan](../../../docs/progress/1.0/jumbling-plan.md) (9 October 2026), not reviewed yet.** It computes how the grip set of the 600-cell grows under jumble twists, for candidate twist menus. The results are leads for the J3 finiteness question and the owner's twist-menu decision, not proofs.

Evidence kind: synthetic geometry, computed in a Linux cloud session. Nothing here is Windows, Direct3D, input or performance evidence.

Sources: the published formal definition of grip theory ([hypercubing.xyz, jumbling](https://hypercubing.xyz/theory/grip-theory/formal/#jumbling)) and this repository (`h4.py`, `exact.py`, `results.json`, `state-contract.md`). HactarCE's Jambler has no licence file, so none of its code was read, copied or translated.

## Files

| File | What it is |
| --- | --- |
| `explore.py` | The closure search, command line, self-test, preset and results tables. Python standard library and NumPy only. |
| `explorer-results.json` | Results of `--preset`: every menu, both closures, per-depth counts, separations and exact checks. |
| `points/*.json` | Viewer samples per run (positions within 50° of n₀ by stereographic projection, int16 base64), and `lattice.json`. |
| `index.html`, `explorer.js` | WebGL viewer (three.js 0.160.0 from jsDelivr): orbit samples coloured by depth, lattice poles, menu picker, depth slider, growth chart, level table. |
| `check_page.mjs` | Headless page check with Playwright and SwiftShader; writes `shots/*.png`. |
| `shots/*.png` | Page check screenshots: desktop light (class-00, lattice closure), desktop dark (plane-10, every-grip closure), phone light (class-05). |

## Definitions

**Geometry.** Everything is built from the 120 unit quaternions of the binary icosahedral group 2I (`h4.py`), as in `jumble_study.py`.
- The 600 poles are the cell centres; the base pole n₀ is cell 0, and its face neighbour is pole 1. A pole's exact coordinates are the sum of its cell's four vertices, in Q(√5)⁴; all poles have the same norm.
- K⁺ is the group of the 7,200 rotations x ↦ l x r with l, r ∈ 2I. A4 is its subgroup of the 12 rotations that fix n₀.
- Each pole c has a fixed frame P_c ∈ K⁺ with P_c n₀ = n_c (P₀ = I). All of these are exact over Q(√5).

**Grips and twists.**
- A grip is a rotation P ∈ SO(4). Its position is P n₀ and its frame class is the coset P·A4. Two grips are equal when they have the same position and P⁻¹P′ ∈ A4.
- The 600 lattice grips are the classes P_c·A4.
- A menu M is a finite set of rotations that fix n₀, given in the base frame. It contains A4 and is closed under inverses and A4-conjugation: the given generators are closed this way. The twist of grip P by m ∈ M is P m P⁻¹. Because of the A4-closure, the twists of a lattice grip do not depend on which of its 12 K⁺ frames is used (plan amendment A4: the menu is conjugated by K⁺).
- Interaction (ball approximation): a twist T of grip c moves a grip e when the angle between their positions is less than θ, and the image is T·P_e. The default θ = 46.8° lies between the 44.478° shell, the last one that meets a cap at the retained depth, and the 49.118° shell (study result 1). The moved set includes c itself and any grip at c's position.

**Closures.** Depth 0 is the 600 lattice grips. Depth d adds every image of a pair (c, e) with both of depth at most d − 1, at least one of them of depth d − 1, that is not already present.
- **lattice**: only the 600 lattice grips twist. This follows the state contract (section 1: grips are world-fixed; cut planes carried by moved pieces are not grips) and the formal definition, where twists use a fixed set of allowed axes and moved grips are stored grips. Here a grip is identified by its position alone, because only its position matters for later twists.
- **all**: every grip in the set twists with its own frame-conjugated menu. Grips are identified by position and frame class. This is the frame model of the J4 brief.
- A run is **closed** (finite) when a complete level adds nothing. It is **growing** when it still adds grips at the depth limit, or when the size budget (orbit representatives) or the time limit cuts a level off.

**Symmetry reduction.** Both closures are K⁺-invariant level by level, because the menus are K⁺-conjugated.
- The search stores one representative per K⁺-orbit. A position is mapped into the Voronoi cell of n₀ (via its nearest pole), and then by A4 to the image with the largest score against a fixed generic vector. Near-ties within 10⁻⁹ look up every alternative, and the alternatives that fix a representative give its stabiliser order.
- Grips are counted as Σ 7200 / |Stab|; positions are counted the same way.
- In the lattice closure the A4 twists are skipped: P_c a P_c⁻¹ lies in K⁺, so its images stay in the same K⁺-orbit.
- `--selftest` compares this with a direct search without reduction, over all 600 twisting grips and every menu element. They agree:
  - lattice closure: class-00 and class-05 up to depth 2, plane-10 at depth 1;
  - all-grips closure at depth 1, grips and positions: class-00 and plane-10.

**Menus.**
- `a4` is the control.
- `class-00` … `class-32` have one generator per realignment class of `../results.json`.
  - The float `R_perp` of the study is made exact from two aligned poles w₁ → u₁ and w₂ → u₂ with independent components: R = [n₀, p_u₁, p_u₂, ×(n₀, p_u₁, p_u₂)] · [n₀, p_w₁, p_w₂, ×(n₀, p_w₁, p_w₂)]⁻¹, where × is the four-dimensional cross product.
  - Exact checks: Rᵀ R = I, det R = 1 and R n₀ = n₀; every pole the float rotation aligns maps exactly onto its target pole; the float and exact rotations agree to 10⁻⁹.
  - All 33 classes are exact over Q(√5).
- `plane-10`, `plane-36` and `plane-72` rotate the plane orthogonal to span(n₀, n₁) by about 10°, 36° and 72°. They use the Cayley form of `witness.py`, with a rational parameter s of denominator at most 1,000:
  - plane-10: s = 15/629, 9.999596°;
  - plane-36: s = 86/971, 35.999747°;
  - plane-72: s = 142/717, 72.000213°.

  Exact 36° and 72° plane rotations are not in Q(√5) here, because tan(θ/2)/√m is not. `plane-36-float` is the exact angle in floating point.

## Exact and float

- **Exact in Q(√5):**
  - the poles, K⁺, A4, the frames and every menu except `plane-36-float`;
  - every recorded coincidence, up to 2,000 per run as a uniform sample: the search records each merged image with its derivation chain, and the image is rebuilt exactly and compared with the stored representative (frames modulo A4 in the all-grips closure);
  - the closest distinct pair of every level with a separation figure, which is checked to be exactly distinct.
- **Float:**
  - the search itself, with positions merged within 10⁻⁹ (chord) and frames within 10⁻⁸ (entrywise);
  - the interaction test, canonicalisation, separations and viewer samples.
- What this means:
  - a merge made by the float search is confirmed exactly for the sample;
  - two grips kept apart are distinct because the smallest separation is many orders above the tolerance, and the closest pair is exactly distinct.

## Reproduce

From the repository root:

```text
python research/jumbling/explorer/explore.py --list-menus
python research/jumbling/explorer/explore.py --selftest
python research/jumbling/explorer/explore.py --menu class-05 --depth 4                 # one run, JSON to stdout
python research/jumbling/explorer/explore.py --menu plane-10 --twisting all --depth 2 --time-limit 120
python research/jumbling/explorer/explore.py --preset --workers 4                      # all runs, writes the result files
python research/jumbling/explorer/explore.py --table                                    # the tables below
```

Options: `--threshold` (degrees, default 46.8), `--budget` (orbit representatives, default 150,000), `--time-limit` (seconds), `--exact-cap` (coincidences checked exactly per run, default 2,000).

The preset uses depth limit 8, a budget of 150,000 representatives and a time limit of 600 s for the lattice closure. For the all-grips closure it uses depth limit 3, 60,000 representatives and 300 s. Separations are computed for complete levels of at most 50,000 representatives.

Page check (Chromium is preinstalled; no `playwright install`):

```text
NODE_PATH=$(npm root -g) PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers \
  node research/jumbling/explorer/check_page.mjs [--vendor DIR]
```

`--vendor` answers the three.js and Google Fonts requests from local copies, for sandboxes without direct CDN access.

## Results

Preset run of 9 October 2026: θ = 46.8°, 4 worker processes, 1,200 s wall time, in a Linux cloud session whose CPU was shared with another job. `python research/jumbling/explorer/explore.py --table` prints these tables from `explorer-results.json`.

How to read the columns:
- **Jumble twists**: menu elements outside A4.
- **Aligned poles**: of the 56 poles nearest n₀, those that the generator maps exactly onto poles (from `../results.json`).
- **Complete depth**: the deepest level the search finished.
- **Cumulative grips per depth**: the number of grips on S³ after each complete level, from depth 0 (not the number of orbit representatives). In the lattice closure, grips and positions coincide. A final entry "≥ N" is the level that the size budget or the time limit cut off, so N is only a lower bound.
- **Result**: "closed" when a complete level added nothing. "×r" is the number of new grips at the last complete level divided by the number at the level before.
- **Min separation**: the smallest angle between two distinct grip positions, at the deepest level with a separation figure (dN). "exact" means that the closest pair was checked to be exactly distinct.
- **Exact checks**: sampled coincidences confirmed in Q(√5), out of those checked.

### Closure: 600 lattice grips twist

| Menu | Jumble twists | Aligned poles | Complete depth | Cumulative grips per depth | Result | Min separation | Exact checks |
| --- | ---: | ---: | ---: | --- | --- | ---: | --- |
| A4 only (control) | 0 | – | 1 | 600 → 600 | closed | – | – |
| class 00: 90.00°, 24 aligned | 6 | 24 | 5 | 600 → 10,200 → 1.64e5 → 2.62e6 → 4.19e7 → 6.71e8 → ≥1.09e9 | growing (×16) | 0.0872° (d4, exact) | 2,000/2,000 |
| class 01: 44.48°, 20 aligned | 8 | 20 | 5 | 600 → 10,200 → 1.64e5 → 2.62e6 → 4.19e7 → 6.71e8 → ≥1.21e9 | growing (×16) | 0.0872° (d4, exact) | 2,000/2,000 |
| class 02: 44.48°, 20 aligned | 8 | 20 | 5 | 600 → 10,200 → 1.64e5 → 2.62e6 → 4.19e7 → 6.71e8 → ≥1.21e9 | growing (×16) | 0.0872° (d4, exact) | 2,000/2,000 |
| class 03: 72.00°, 12 aligned | 12 | 12 | 5 | 600 → 10,200 → 1.64e5 → 2.62e6 → 4.19e7 → 6.71e8 → ≥1.1e9 | growing (×16) | 0.0872° (d4, exact) | 2,000/2,000 |
| class 04: 72.00°, 12 aligned | 12 | 12 | 5 | 600 → 10,200 → 1.64e5 → 2.62e6 → 4.19e7 → 6.71e8 → ≥1.13e9 | growing (×16) | 0.0872° (d4, exact) | 2,000/2,000 |
| class 05: 63.43°, 8 aligned | 6 | 8 | 4 | 600 → 18,600 → 4.69e5 → 1.17e7 → 2.93e8 → ≥1.17e9 | growing (×25) | 0.0345° (d4, exact) | 2,000/2,000 |
| class 06: 63.98°, 8 aligned | 24 | 8 | 3 | 600 → 58,200 → 6.08e6 → 6.23e8 → ≥1.27e9 | growing (×102) | 0.152° (d2, exact) | 2,000/2,000 |
| class 07: 65.34°, 6 aligned | 24 | 6 | 2 | 600 → 79,800 → 9.52e6 → ≥1.08e9 | growing (×119) | 0.171° (d2, exact) | 2,000/2,000 |
| class 08: 26.57°, 4 aligned | 6 | 4 | 2 | 600 → 1.27e5 → 2.8e7 → ≥1.1e9 | growing (×221) | 0.0431° (d2, exact) | 2,000/2,000 |
| class 09: 41.81°, 4 aligned | 6 | 4 | 3 | 600 → 54,600 → 4.43e6 → 3.57e8 → ≥1.08e9 | growing (×80.7) | 0.0197° (d3, exact) | 2,000/2,000 |
| class 10: 47.21°, 4 aligned | 12 | 4 | 2 | 600 → 2.6e5 → 1.44e8 → ≥1.16e9 | growing (×553) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 11: 61.73°, 4 aligned | 24 | 4 | 2 | 600 → 2.37e5 → 7.2e7 → ≥1.18e9 | growing (×304) | 0.0431° (d2, exact) | 2,000/2,000 |
| class 12: 61.73°, 4 aligned | 24 | 4 | 2 | 600 → 2.37e5 → 7.2e7 → ≥1.18e9 | growing (×304) | 0.0431° (d2, exact) | 2,000/2,000 |
| class 13: 61.04°, 4 aligned | 24 | 4 | 2 | 600 → 2.81e5 → 1.59e8 → ≥1.18e9 | growing (×566) | 0.0101° (d2, exact) | 2,000/2,000 |
| class 14: 27.95°, 3 aligned | 24 | 3 | 2 | 600 → 3.7e5 → 2.63e8 → ≥1.59e9 | growing (×710) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 15: 27.95°, 3 aligned | 24 | 3 | 2 | 600 → 3.7e5 → 2.63e8 → ≥1.59e9 | growing (×710) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 16: 31.21°, 3 aligned | 24 | 3 | 2 | 600 → 4.57e5 → 3.63e8 → ≥1.68e9 | growing (×796) | 0.396° (d1, exact) | 2,000/2,000 |
| class 17: 31.21°, 3 aligned | 24 | 3 | 2 | 600 → 4.57e5 → 3.63e8 → ≥1.68e9 | growing (×796) | 0.396° (d1, exact) | 2,000/2,000 |
| class 18: 51.61°, 3 aligned | 24 | 3 | 2 | 600 → 4.47e5 → 3.58e8 → ≥1.69e9 | growing (×801) | 0.0101° (d2, exact) | 2,000/2,000 |
| class 19: 51.61°, 3 aligned | 24 | 3 | 2 | 600 → 4.47e5 → 3.58e8 → ≥1.69e9 | growing (×801) | 0.0101° (d2, exact) | 2,000/2,000 |
| class 20: 56.01°, 3 aligned | 24 | 3 | 2 | 600 → 4.28e5 → 3.4e8 → ≥1.65e9 | growing (×796) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 21: 56.01°, 3 aligned | 24 | 3 | 2 | 600 → 4.28e5 → 3.4e8 → ≥1.68e9 | growing (×796) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 22: 56.01°, 3 aligned | 24 | 3 | 2 | 600 → 4.28e5 → 3.4e8 → ≥1.68e9 | growing (×796) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 23: 56.01°, 3 aligned | 24 | 3 | 2 | 600 → 4.28e5 → 3.4e8 → ≥1.65e9 | growing (×796) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 24: 68.04°, 3 aligned | 24 | 3 | 2 | 600 → 3.77e5 → 2.64e8 → ≥1.61e9 | growing (×701) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 25: 68.04°, 3 aligned | 24 | 3 | 2 | 600 → 3.77e5 → 2.64e8 → ≥1.61e9 | growing (×701) | 0.00897° (d2, exact) | 2,000/2,000 |
| class 26: 38.72°, 2 aligned | 24 | 2 | 2 | 600 → 4.07e5 → 2.75e8 → ≥1.63e9 | growing (×674) | 0.0197° (d2, exact) | 2,000/2,000 |
| class 27: 38.72°, 2 aligned | 24 | 2 | 2 | 600 → 4.07e5 → 2.75e8 → ≥1.63e9 | growing (×674) | 0.0197° (d2, exact) | 2,000/2,000 |
| class 28: 39.00°, 2 aligned | 24 | 2 | 2 | 600 → 3.14e5 → 1.63e8 → ≥1.42e9 | growing (×520) | 0.0148° (d2, exact) | 2,000/2,000 |
| class 29: 39.00°, 2 aligned | 24 | 2 | 2 | 600 → 3.14e5 → 1.63e8 → ≥1.42e9 | growing (×520) | 0.0148° (d2, exact) | 2,000/2,000 |
| class 30: 50.49°, 2 aligned | 24 | 2 | 2 | 600 → 1.64e5 → 4.06e7 → ≥1.32e9 | growing (×248) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 31: 50.49°, 2 aligned | 24 | 2 | 2 | 600 → 1.64e5 → 4.06e7 → ≥1.32e9 | growing (×248) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 32: 71.22°, 2 aligned | 24 | 2 | 2 | 600 → 1.57e5 → 3.94e7 → ≥1.18e9 | growing (×251) | 0.0442° (d2, exact) | 2,000/2,000 |
| plane 10° (exact Cayley, 9.99960°) | 8 | – | 2 | 600 → 1.09e5 → 2.03e7 → ≥1.08e9 | growing (×187) | 0.00172° (d2, exact) | 2,000/2,000 |
| plane 36° (exact Cayley, 35.99975°) | 8 | – | 2 | 600 → 1.09e5 → 2.04e7 → ≥1.09e9 | growing (×188) | 0.00417° (d2, exact) | 2,000/2,000 |
| plane 72° (exact Cayley, 72.00021°) | 8 | – | 2 | 600 → 1.09e5 → 1.99e7 → ≥1.14e9 | growing (×183) | 0.0071° (d2, exact) | 2,000/2,000 |
| plane 36° (float, exact angle) | 8 | – | 2 | 600 → 1.09e5 → 2.04e7 → ≥1.09e9 | growing (×188) | 0.0044° (d2) | float menu |

### Closure: every grip twists

| Menu | Jumble twists | Aligned poles | Complete depth | Cumulative grips per depth | Result | Min separation | Exact checks |
| --- | ---: | ---: | ---: | --- | --- | ---: | --- |
| A4 only (control) | 0 | – | 1 | 600 → 600 | closed | – | – |
| class 00: 90.00°, 24 aligned | 6 | 24 | 2 | 600 → 15,600 → 4.07e7 → ≥1.55e8 | growing (×2.72e+03) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 01: 44.48°, 20 aligned | 8 | 20 | 2 | 600 → 15,600 → 2.81e7 → ≥7.86e7 | growing (×1.87e+03) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 02: 44.48°, 20 aligned | 8 | 20 | 2 | 600 → 15,600 → 2.81e7 → ≥7.83e7 | growing (×1.87e+03) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 03: 72.00°, 12 aligned | 12 | 12 | 2 | 600 → 12,600 → 1.19e7 → ≥1.38e8 | growing (×992) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 04: 72.00°, 12 aligned | 12 | 12 | 2 | 600 → 12,600 → 1.19e7 → ≥1.35e8 | growing (×992) | 0.0872° (d2, exact) | 2,000/2,000 |
| class 05: 63.43°, 8 aligned | 6 | 8 | 2 | 600 → 22,200 → 9.7e7 → ≥3.76e8 | growing (×4.49e+03) | 0.0345° (d2, exact) | 2,000/2,000 |
| class 06: 63.98°, 8 aligned | 24 | 8 | 1 | 600 → 65,400 → ≥4.76e8 | growing | 1.53° (d1, exact) | 2,000/2,000 |
| class 07: 65.34°, 6 aligned | 24 | 6 | 1 | 600 → 87,000 → ≥5.04e8 | growing | 2.07° (d1, exact) | 2,000/2,000 |
| class 08: 26.57°, 4 aligned | 6 | 4 | 1 | 600 → 2.06e5 → ≥4.37e8 | growing | 0.817° (d1, exact) | 2,000/2,000 |
| class 09: 41.81°, 4 aligned | 6 | 4 | 1 | 600 → 58,200 → ≥4.67e8 | growing | 1.97° (d1, exact) | 2,000/2,000 |
| class 10: 47.21°, 4 aligned | 12 | 4 | 1 | 600 → 4.11e5 → ≥4.49e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 11: 61.73°, 4 aligned | 24 | 4 | 1 | 600 → 3.71e5 → ≥5.3e8 | growing | 0.817° (d1, exact) | 2,000/2,000 |
| class 12: 61.73°, 4 aligned | 24 | 4 | 1 | 600 → 3.71e5 → ≥5.33e8 | growing | 0.817° (d1, exact) | 2,000/2,000 |
| class 13: 61.04°, 4 aligned | 24 | 4 | 1 | 600 → 4.11e5 → ≥5.61e8 | growing | 0.396° (d1, exact) | 2,000/2,000 |
| class 14: 27.95°, 3 aligned | 24 | 3 | 1 | 600 → 5.55e5 → ≥4.39e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 15: 27.95°, 3 aligned | 24 | 3 | 1 | 600 → 5.55e5 → ≥4.39e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 16: 31.21°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.33e8 | growing | 0.396° (d1, exact) | 2,000/2,000 |
| class 17: 31.21°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.33e8 | growing | 0.396° (d1, exact) | 2,000/2,000 |
| class 18: 51.61°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.32e8 | growing | 0.396° (d1, exact) | 2,000/2,000 |
| class 19: 51.61°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.4e8 | growing | 0.396° (d1, exact) | 2,000/2,000 |
| class 20: 56.01°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.38e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 21: 56.01°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.35e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 22: 56.01°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.35e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 23: 56.01°, 3 aligned | 24 | 3 | 1 | 600 → 6.56e5 → ≥4.38e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 24: 68.04°, 3 aligned | 24 | 3 | 1 | 600 → 5.55e5 → ≥4.38e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 25: 68.04°, 3 aligned | 24 | 3 | 1 | 600 → 5.55e5 → ≥4.38e8 | growing | 0.474° (d1, exact) | 2,000/2,000 |
| class 26: 38.72°, 2 aligned | 24 | 2 | 1 | 600 → 4.22e5 → ≥6.17e8 | growing | 0.552° (d1, exact) | 2,000/2,000 |
| class 27: 38.72°, 2 aligned | 24 | 2 | 1 | 600 → 4.22e5 → ≥6.17e8 | growing | 0.552° (d1, exact) | 2,000/2,000 |
| class 28: 39.00°, 2 aligned | 24 | 2 | 1 | 600 → 4.36e5 → ≥5.23e8 | growing | 0.609° (d1, exact) | 2,000/2,000 |
| class 29: 39.00°, 2 aligned | 24 | 2 | 1 | 600 → 4.36e5 → ≥5.23e8 | growing | 0.609° (d1, exact) | 2,000/2,000 |
| class 30: 50.49°, 2 aligned | 24 | 2 | 1 | 600 → 2.36e5 → ≥4.45e8 | growing | 0.913° (d1, exact) | 2,000/2,000 |
| class 31: 50.49°, 2 aligned | 24 | 2 | 1 | 600 → 2.36e5 → ≥4.46e8 | growing | 0.913° (d1, exact) | 2,000/2,000 |
| class 32: 71.22°, 2 aligned | 24 | 2 | 1 | 600 → 1.64e5 → ≥5.5e8 | growing | 1.05° (d1, exact) | 2,000/2,000 |
| plane 10° (exact Cayley, 9.99960°) | 8 | – | 1 | 600 → 1.13e5 → ≥4.33e8 | growing | 0.41° (d1, exact) | 2,000/2,000 |
| plane 36° (exact Cayley, 35.99975°) | 8 | – | 1 | 600 → 1.13e5 → ≥4.51e8 | growing | 0.365° (d1, exact) | 2,000/2,000 |
| plane 72° (exact Cayley, 72.00021°) | 8 | – | 1 | 600 → 1.13e5 → ≥4.44e8 | growing | 0.102° (d1, exact) | 2,000/2,000 |
| plane 36° (float, exact angle) | 8 | – | 1 | 600 → 1.13e5 → ≥4.51e8 | growing | 0.365° (d1) | float menu |

### Observations

These are leads for J3 and the menu decision, not proofs.

- **The control closes.** With A4 alone, no grips are added in either closure: the orbit stays at the 600 lattice grips.
- **Every jumble menu grows.** In both closures, none of the 33 realignment classes and none of the plane menus closes before the size budget or the time limit. Every sampled coincidence was confirmed exactly, and every closest pair with a separation figure is exactly distinct. So the growth is not an effect of the float tolerance.
- **Exact geometric growth for the classes with many aligned poles** (lattice closure, all complete levels):
  - class-00 to class-04 have 40·(16^(d+1) − 1) grips after depth d, for d = 0 to 5: each level adds exactly 16 times as many grips as the level before.
  - class-05 has 150·(5^(2d+1) − 1) grips after depth d, for d = 0 to 4: each level adds 25 times as many.
  - Agreeing counts do not mean agreeing sets. The depth-1 position sets of class-00 to class-03 coincide, and class-04 differs. From depth 2 on, all five sets differ, although the counts agree.
- **Fewer aligned poles, faster growth.**
  - class-06 to class-32 have 2 to 8 aligned poles. They add about 80 to 800 times as many grips at their last complete level as at the level before.
  - Most of them pass about 10⁹ grips by depth 3.
  - Classes with the same generator angle and aligned-pole count give equal counts at every complete level: 01 and 02, 03 and 04, 11 and 12, 14 and 15, 16 and 17, 18 and 19, 20 and 23, 21 and 22, 24 and 25, 26 and 27, 28 and 29, 30 and 31. A mirror symmetry between them is a likely explanation; it is not checked here.
- **The orbit becomes dense on the scale of the puzzle.** The smallest separation falls by a factor of about 3 to 6 per level, together with the median nearest-neighbour angle:
  - class-00: 4.78°, 0.913°, 0.282°, 0.087° at depths 1 to 4;
  - class-05: 4.28°, 0.731°, 0.125°, 0.034°.

  Most other menus are already below 0.05° at depth 2, and plane-10 is at 0.0017°. The interaction angle is 46.8°.
- **The every-grip closure grows faster.**
  - class-00 has 4.07·10⁷ grips at depth 2, against 1.64·10⁵ in the lattice closure.
  - Frames make grips outnumber positions: 15,600 grips at 10,200 positions at depth 1, and 4.07·10⁷ at 2.18·10⁷ at depth 2.
  - Its depth-1 positions equal the lattice closure's depth-1 positions for every menu: only the lattice grips have twisted by then.
- **The plane menus barely depend on the angle at depth 1.** plane-10, plane-36 and plane-72 all give 108,600 grips (113,400 in the every-grip closure); they differ from depth 2 on.
- **plane-36 exact and float agree.** The exact plane-36 menu (s = 86/971, 0.00025° off) and `plane-36-float` give the same counts at every complete level in both closures. Their smallest separations differ slightly (0.00417° and 0.0044° at depth 2), as expected from the small angle difference.

### Limits

- **Ball approximation.** A twist moves every grip whose position lies within θ of the twisting grip. The exact cap and piece geometry is not modelled: which pieces a twist carries, and whether a moved cut still allows a twist. The results depend on θ; use `--threshold` to vary it.
- **No legality or shape condition.** Every grip is assumed twistable in every state, and the change of the outer shape after a jumble twist is ignored. This is an over-approximation of the grips a real puzzle reaches in the ball model.
- **Growth is not a proof.** Growth within the budget does not prove that a set is infinite, and the growth factors are fitted from at most six levels. Only "closed" is a finite result, and only the control is closed.
- **Exact checks are a sample.** At most 2,000 coincidences per run are rebuilt and checked exactly, out of up to 1.1·10⁷ coincidences in a run. The other merges rest on the 10⁻⁹ tolerance. The closest pair is checked only at levels with a separation figure.
- **Cut levels depend on the machine.** The size budget is deterministic. The every-grip runs of class-00 to class-05, however, stopped at the 300 s time limit on a shared CPU, so their depth-3 counts are lower bounds that depend on the machine. Every cut level is a partial set, whose size depends on the processing order.
- **Separation coverage.** Separations are computed only for complete levels with at most 50,000 representatives. They are distances between positions; frames are compared only for identity.
- **Viewer samples.** The page shows at most 1,500 sampled grips per depth within 50° of n₀, not the full set.
- **One generator per class.** Each realignment class is represented by one exact generator, together with its A4-conjugates and inverses.
