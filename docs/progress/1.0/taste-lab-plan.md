# Taste Lab phase 1: plan

Status: **revised after the plan check** (call 20261002T171939Z-9e60ff72, findings TL-P01 to TL-P09, all adopted) and its scoped re-check (call 20261002T174128Z-223bc76c, TL-P03, TL-P07 and TL-P09 tightened); the palette rule changed after the feasibility prototype (section 4.3). Owner decision of 2 October 2026 ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)): Taste Lab is built now, its front end is a private Artifact page on claude.ai, and its model learns from few pairwise choices and draws new variants itself.

## 1. Goal and acceptance

Give the owner a private page on which a model learns the owner's visual preferences for the 600-cell from a few dozen pairwise choices, and hands the result to the design track (G2 to G5) as starting points.

Phase 1 is accepted when:
1. the core and geometry tests pass headless (`python tests/test_tastelab.py`, which runs the node tests of both folders);
2. the palette feasibility sweep (section 4.3) finds feasible looks under the hard checks and reports the rejection rate;
3. the simulated-owner experiment (section 6) meets its pass criteria; if it does not, the result goes to the owner before the learner is offered as one;
4. the canary (section 7.1) and the final page pass their Artifact checks, and one functional pass (record a comparison, read it back with `ArtifactData`) succeeds;
5. no class B image, personal data, key or private path is in the page, its storage or the repository.

## 2. Scope

- **Phase 1a (this plan):** the parameter studio. No downloads, no installs, no external sources: the page draws every look itself.
- **Phase 1b (later, one owner approval first):** the class A image swipe. It needs an embedding model (about 1 GB of weights) and a deep-learning runtime on the allowlist, plus the class A source domains. These are "Ask the owner" items (allowlist sources, large model download), even in the cloud container. The page reserves a tab for it.
- **Out of scope:** class B images (phase 2, local only); sound; shipping a look tuner to end users (a 1.0 candidate for stage 2.2); the full-detail renderer (Look Lab, G3); the mechanical model. The preview is synthetic geometry of the 600-cell, not the 259,800-slot puzzle, and says so.

## 3. Parts

| Part | Path | Owner | Notes |
|---|---|---|---|
| Colour core | `tools/tastelab/core/color.js` | Codex packet P1 | sRGB, linear RGB and OKLab conversion; gamut mapping by chroma reduction at constant L and hue; ΔE in OKLab; colour-vision-deficiency simulation (protan, deutan, tritan; Machado 2009, severity 1.0, applied in linear RGB) |
| Preference model | `tools/tastelab/core/gp.js` | Codex packet P1 | section 5 |
| Next-pair choice | `tools/tastelab/core/acquire.js` | Codex packet P1 | section 5.4 |
| Parameter space and checks | `tools/tastelab/core/space.js` | Codex packet P1 | section 4 |
| Core tests | `tools/tastelab/core/tests/*.test.mjs` | Codex packet P1 | acceptance: `node --test "tools/tastelab/core/tests/*.test.mjs"` |
| Geometry | `tools/tastelab/page/geometry.js` | Codex packet P2 | section 4.4 |
| Geometry tests | `tools/tastelab/page/tests/*.test.mjs`, `tools/tastelab/page/tests/fixtures/600cell.json` | Codex packet P2 | acceptance: `node --test "tools/tastelab/page/tests/*.test.mjs"` |
| Test runner | `tests/test_tastelab.py` | Claude | runs `node --test` on the test files of both folders and checks the geometry fixture; skips with a clear message when Node.js is missing |
| Preview | `tools/tastelab/page/preview.js` | Claude | WebGL 2: perspective projection from 4D, cells shrunk by the gap parameter, edges, gloss, glow, fog; one animated cap turn (section 4.4) with the chosen duration and easing |
| Page | `tools/tastelab/page/index.html`, `app.js`, `worker.js` | Claude | two looks side by side, keys A, B, S (same), X (both bad), Z (undo), "what matters" panel, scene switch, export; model work in a Web Worker |
| Sweep and experiment | `tools/tastelab/sim/sweep.mjs`, `tools/tastelab/sim/experiment.mjs` | Claude | section 4.3 and section 6 |

P1 and P2 own disjoint files. Each packet carries its interfaces (exported functions, argument and return shapes) in its text; no skeleton is committed before review. If the Artifact runtime does not serve supporting modules (section 7.1), a small build script inlines them into one HTML file.

## 4. Parameter space (first proposal; the owner edits it before the first session)

| Group | Parameter | Range | Encoding |
|---|---|---|---|
| Palette | hue rotation | 0 to 360 degrees | circular (cos, sin) |
| Palette | hue spread | 60 to 360 degrees | linear |
| Palette | lightness (OKLab L) | 0.45 to 0.85 | linear |
| Palette | lightness alternation | 0 to 0.2 | linear |
| Palette | colour classes k | 4 to 8 | integer |
| Palette | chroma (OKLab C) | 0.04 to 0.20 | linear |
| Background | lightness, tint hue, tint strength | 0.05 to 0.95; 0 to 360; 0 to 0.05 | linear; circular; linear |
| Geometry | sticker gap, edge weight, edge brightness | 0 to 0.3; 0 to 3 px; 0 to 1 | linear |
| Material | gloss, glow, fog density | 0 to 1 each | linear |
| Motion | turn duration; easing shape (two numbers of one curve family) | 150 to 900 ms; 0 to 1 each | linear |

### 4.1 Scenes

Solving, inspecting, celebrating. The model is one joint Gaussian process over (look, scene). The kernel is k((x,s),(x',s')) = k_x(x,x') · k_s(s,s') with k_s = 1 for the same scene and ρ otherwise; ρ in [0, 1] is fitted. Looks are shared where the owner's choices agree across scenes and differ where they do not. All hyperparameters are shared and stored once.

### 4.2 Palette rule and hard checks

- Cells are coloured by a structural rule: the 600 cells form 20 rings of 30 cells, and a proper colouring of the ring adjacency graph (rings that share a face get different classes) assigns each ring one of k colour classes. Class i has hue = rotation + spread · i / k, lightness = L ± alternation (alternating by i), chroma C. The real sticker palette is G4's job; the studio learns the palette character.
- One function maps every OKLCh colour into sRGB by reducing chroma at constant L and hue. It returns a failure, not a colour, when no chroma at that L fits (in particular when the derived L is outside 0 to 1). A look with any failed class colour is invalid and is rejected before rendering and before any other check. The renderer and the checks use the same mapped colour.
- Hard checks, on the mapped colours: touching classes (rings that share a face) differ by at least the ΔE threshold in OKLab under normal vision and each simulated deficiency (first value 0.08, to be set in G4); every class differs from the background by at least 0.20 in OKLab L; parameters within range.
- A failing look is never shown. Thresholds are never relaxed automatically.

### 4.3 Feasibility sweep (before the model)

A prototype (pure Python, 4,096 random points, Machado severity 1.0) showed that 20 distinct ring colours cannot meet the 0.08 threshold: no point passed, the best worst-view minimum ΔE was 0.015 with the hue-order assignment and 0.066 with a searched assignment, and deuteranopia was the limiting view in most points. The ring adjacency graph of a 7-regular ring cover (each ring touches 7 others) has chromatic number 4; with k = 4 to 8 classes the best worst-view minimum ΔE was 0.14 to 0.16. `tools/tastelab/sim/sweep.mjs` must reproduce this result before the model is built. Phase 1 therefore colours by graph colouring with k classes; the threshold is unchanged. The ring cover is the one in which every ring touches exactly 7 others.

Evaluate 4,096 Sobol points of the palette and background parameters under the hard checks. Report the rejection rate and the feasible region, and keep the feasible set as a fallback pool. If no look is feasible, stop and give the owner the options (threshold, palette rule). If a round finds no feasible candidate, the page draws from the fallback pool and says so.

### 4.4 Geometry and the cap turn

- 120 vertices as unit quaternions (the binary icosahedral group), 720 edges, 1,200 triangles, 600 tetrahedral cells, face adjacency.
- Rings: the 600 cells split into 20 disjoint rings of 30 face-connected tetrahedra (Boerdijk–Coxeter helices). The construction is stated in the code with its source.
- Cap turn (illustrative): a rotation of 4D space that fixes the chosen cell's centre direction c and rotates the orthogonal 3-space by a symmetry of the cell's tetrahedron (order 3 about a vertex axis). The cap is the set of cells whose centre has dot product with c above a stated cut. The animation interpolates the angle.
- Tests: the vertex set equals a fixture generated from `research/audit/verify_regular_geometry.py`'s construction; unit norms; edge length 1/φ; each cell has 4 face neighbours; the rings are disjoint, cover all 600 cells, and each is a closed face-connected cycle; every intermediate turn matrix is orthogonal and fixes c; the endpoint maps the cap's vertex set onto itself.

## 5. Model (theory in the owner's guide, sections 8.20 and 8.21)

### 5.1 Scale

The probit noise is fixed at σ = 1, which fixes the utility scale (the likelihood depends only on signal/σ). The fitted hyperparameters are the signal amplitude, the ARD length scales, ρ and the tie threshold, each with a log-normal prior and bounds (length scales 0.05 to 5 on the normalized axes).

### 5.2 Observations

With d = f(A) − f(B) and a tie threshold ε > 0 (fitted), the three outcomes are normalized:
- P(A over B) = Φ((d − ε)/√2),
- P(same) = Φ((ε − d)/√2) − Φ((−ε − d)/√2),
- P(B over A) = Φ((−d − ε)/√2).

"Both bad" adds no preference observation in phase 1. It marks both looks as rejected: candidates within a stated radius of a rejected look are excluded, and the next round explores away from the current best. An acceptability model is a later option. Undo removes the last record; the model is refitted from the remaining records, and the tests check that undo equals a fresh fit.

### 5.3 Fit

- Prior: zero-mean GP with the kernel of section 4.1; k_x is ARD squared-exponential on the encoded parameters (hue as cos and sin).
- Update: Laplace approximation by Newton's method after each answer, at most 30 iterations; on non-convergence the previous fit is kept and the page says so.
- Hyperparameters: refitted every 10 answers by maximizing the approximate marginal likelihood plus the priors, at most 60 evaluations.

### 5.4 Next pair

- 512 candidates per round: Sobol points plus local perturbations around the current best, hard checks first.
- One side is the current best. The other is a Thompson sample: one joint posterior draw over the candidates, its best. About 20% of rounds use the pair with the largest expected information about f (mutual information between the three-outcome answer and the latent utilities, estimated by sampling); about 5% repeat an earlier pair.

### 5.5 Limits

- At most 400 latent looks in total (all scenes). Pruning first removes candidates that were never shown. When the shown looks alone reach 400, the fit uses the comparisons among the 400 most recently shown looks; older comparisons stay stored and exported but leave the fit, and the page says how many.
- Work runs in a Web Worker. Target: the next pair is ready within 1 s at the maximum size. The benchmark in headless Chromium is indicative only, not a measurement on the owner's laptop.

### 5.6 Settled

"Settled" when, over 200 joint posterior draws of the latent utilities, the best beats every candidate of its round in at least 90% of draws, the best is unchanged for 15 answers, and two separate sessions end in the same region.

## 6. Simulated-owner experiment

- Synthetic utilities with a known optimum: a smooth function of three to five parameters, the others irrelevant, with scene-specific and shared parts; its optimum lies inside the feasible set of section 4.3.
- Answers use the observation model of section 5.2 at three noise levels (signal-to-noise 1, 2 and 4) and a fixed tie threshold.
- Measure normalized regret r = (u* − u(best)) / (u* − median u over the feasible sweep), not distance to one optimum vector.
- Run a low-dimensional baseline first, then the full space. At least 50 paired seeds per noise level; random pairs with the same budget as the baseline method.
- Pass: at 60 comparisons and the middle noise level, median r ≤ 0.10, the model beats random pairs on paired seeds (one-sided Wilcoxon signed-rank test, p < 0.01), the fitted length scales rank the relevant parameters above the irrelevant ones in at least 80% of seeds, and "settled" is wrong (declared while r > 0.20) in at most 10% of seeds.
- The result is a sanitized report in `docs/progress/1.0/` and sets the expected number of comparisons before the owner starts.

## 7. Storage, privacy and export

- Capabilities: `db` (owner-only rules: every path readable and writable only at owner level), `user`, `downloads` for export. No `sample`, `mcp`, `room` or `comments`.
- Collections: `comparisons` (looks as parameter vectors, scene, answer, time, session), `sessions`, `space` (the edited parameter space), `model` (fitted hyperparameters).
- Claude reads the data with `ArtifactData` and writes a copy to `work/loop-memory/tastelab/` (private, ignored by Git). Export saves the same JSON to the owner's computer.
- Never stored: images, names, keys, anything from the owner's sessions.

### 7.1 Canary first

Before the learner is connected, publish a minimal private canary page with code-contained fixtures. It checks: one supporting module import; one WebGL 2 pixel; persist and reload one fixture; read it through `ArtifactData`; export it with `downloads`; the page's behaviour when a capability is absent or a write fails; the declared owner-only rules as read back. A read or write by a second account cannot be tested from this session; it stays unverified until the owner opens the page from another account, and the report says so. The canary is deleted after the check if the owner agrees.

## 8. Process

1. Scoped re-check of this revised plan (Astra, max, fast tier).
2. Canary (section 7.1) and feasibility sweep (section 4.3).
3. Commit this plan. Then run packets P1 and P2 through `--kind implement` in parallel, each with its interfaces, owned files including tests, acceptance command and stop condition. Nobody commits while they run. Claude writes the runner, preview, page, sweep and experiment.
4. Claude reviews and applies Codex's patches.
5. Checks on the integrated candidate: `python tests/test_tastelab.py`, the simulated-owner experiment, a browser check of the page with the pre-installed Chromium (rendering and keys).
6. One Astra review of the candidate that passed step 5 (behaviour and design), then at most two scoped verification rounds; the checks run again on every changed candidate. Commit only when the checks pass and the review covers that candidate.
7. Publish and the functional pass.
8. Owner: edit the parameter space, then start comparing.
