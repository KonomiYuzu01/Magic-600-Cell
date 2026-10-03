# Taste Lab phase 1: plan

Status: **revised after the plan check** (call 20261002T171939Z-9e60ff72, findings TL-P01 to TL-P09, all adopted) and its scoped re-checks (call 20261002T174128Z-223bc76c, TL-P03, TL-P07 and TL-P09 tightened; call 20261002T174734Z-867f3bb0, passed with no new blocking finding); the palette rule changed after the feasibility prototype (section 4.3). Owner decision of 2 October 2026 ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)): Taste Lab is built now, its front end is a private Artifact page on claude.ai, and its model learns from few pairwise choices and draws new variants itself. The owner design decisions of 2 October 2026 ([owner-decisions-2026-10-02-design](../../wiki/decisions/owner-decisions-2026-10-02-design.md)) changed sections 1, 2, 4, 6 and 7: two theme families, each learned separately (section 4.5); subtler celebrating ranges (section 4.1); an optional one-line reason per answer, the preset export with cost marks, and labels that follow the glossary (section 7). Scoped re-check of these changes: call 20261002T234348Z-2397245e, one major finding (TL-P10: the two-family criterion accepted collapsed results), adopted in section 6; verification round call 20261002T235138Z-5bbb135e: pass. Revised on 3 October 2026: the simulated-owner experiment missed its criteria at 60 comparisons; an escalated diagnosis replaced the learner's kernel, priors, hyperparameter search and pair choice (sections 5.1 and 5.3 to 5.5), and the owner decisions of 3 October 2026 ([owner-decisions-2026-10-03-taste-lab](../../wiki/decisions/owner-decisions-2026-10-03-taste-lab.md)) moved the evaluation points (section 6) and had the settled rule recalibrated (section 5.6).

## 1. Goal and acceptance

Give the owner a private page on which a model learns the owner's visual preferences for the 600-cell from a few dozen pairwise choices, and hands the result to the design track (G2 to G5) as starting points: one look per theme family and scene, six with the default two families.

Phase 1 is accepted when:
1. the core and geometry tests pass headless (`python tests/test_tastelab.py`, which runs the node tests of both folders);
2. the palette feasibility sweep (section 4.3) finds feasible looks under the hard checks and reports the rejection rate;
3. the simulated-owner experiment (section 6) meets its pass criteria; if it does not, the result goes to the owner before the learner is offered as one;
4. the canary (section 7.1) and the final page pass their Artifact checks, and one functional pass (record a comparison, read it back with `ArtifactData`) succeeds;
5. no class B image, personal data, key or private path is in the page, its storage or the repository.

## 2. Scope

- **Phase 1a (this plan):** the parameter studio. No downloads, no installs, no external sources: the page draws every look itself.
- **Phase 1b (later, one owner approval first):** the class A image swipe. It needs an embedding model (about 1 GB of weights) and a deep-learning runtime on the allowlist, plus the class A source domains. These are "Ask the owner" items (allowlist sources, large model download), even in the cloud container. The page reserves a tab for it. The owner chose annotated references and nexus cards (4A), not embedding-first discovery (4C); that decision neither requests nor grants phase 1b, and the reserved tab stays a disabled placeholder.
- **Out of scope:** class B images (phase 2, local only); sound (decision 10B); text budgets (gap 9); shipping a look tuner to end users (a 1.0 candidate for stage 2.2); the full-detail renderer (Look Lab, G3); performance measurement; the mechanical model. The preview is synthetic geometry of the 600-cell, not the 259,800-slot puzzle, and says so.

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
| Geometry | cell gap, edge weight, edge brightness | 0 to 0.3; 0 to 3 px; 0 to 1 | linear |
| Material | gloss, glow, fog density | 0 to 1 each | linear |
| Motion | twist duration; easing shape (two numbers of one curve family) | 150 to 900 ms; 0 to 1 each | linear |

### 4.1 Scenes

Solving, inspecting, celebrating. The model is one joint Gaussian process over (look, scene). The kernel is k((x,s),(x',s')) = k_x(x,x') · k_s(s,s') with k_s = 1 for the same scene and ρ otherwise; ρ in [0, 1] is fitted. Looks are shared where the owner's choices agree across scenes and differ where they do not. All hyperparameters are shared and stored once.

Celebrating is tuned toward subtle, because rewards are small, in place and never modal (owner design decision 1). The table carries scene limits that narrow a parameter's range in one scene; the first values are glow at most 0.5 and twist duration at most 600 ms in the celebrating scene. A scene's candidates stay within its limits, and the encoding keeps the table's ranges, so looks of different scenes stay comparable. The preview has no full-screen effect in any scene: glow is a rim light on each cell, and fog fades distant cells toward the background.

### 4.2 Palette rule and hard checks

- Cells are coloured by a structural rule: the 600 cells form 20 rings of 30 cells, and a proper colouring of the ring adjacency graph (rings that share a face get different classes) assigns each ring one of k colour classes. Class i has hue = rotation + spread · i / k, lightness = L ± alternation (alternating by i), chroma C. The real sticker palette is G4's job; the studio learns the palette character. Colour carries the class, never identity (owner design decision 2); the preview shows no exact IDs, so resolving repeated colours on focus stays with G4 and the product.
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

### 4.5 Theme families

1.0 ships two authored theme families, each with the three scene looks (owner design decision 3). The stored table lists the families (default two, which the owner can rename). The count is a parameter, not a constant: a later scope triage can drop the second family without code changes, and with one family the page and the export work as before.

- **Choice: each family is learned separately** (option b of the hand-off of 2 October). Every answer records its family. Each family has its own model, hyperparameters, look cap and settled state, and reads only its own answers. Reasons: the families cannot blend, however few the answers; the single-family learner, its tests and its experiment carry over unchanged; dropping a family touches no model code. Cost: nothing transfers between families, so each family needs its own comparisons (section 6 states the number).
- **Not chosen.** (a) The family as a context input with its own fitted correlation, like ρ for scenes: it would share answers between families, but the correlation is estimated from few answers, an overestimate pulls the families together, and one correlation cannot say which parameters are shared. (c) The second family starting from a region that contrasts with the first: it presumes which parameters should differ and steers the second family away from the owner's own choice.
- **Shared-attribute summary.** The export lists, per parameter and scene, each family's best value and the difference between the families in units of the parameter's range. A parameter within 0.1 in every scene is marked shared. The summary describes the answers; whether a shared attribute becomes fixed is the design track's decision.
- **Fixed across families:** commands, keys, layout anchors, role glyphs, status meanings, comparison conventions, motion meanings, accessibility alternatives and the projection. None of them is a studio parameter; the page keeps the preview's projection, layout and keys the same for every family. Every studio parameter belongs to what families may vary: palette character (the palette and background groups), surfaces and materials (gloss, glow, fog, edges) and density (cell gap, edge weight). Typography is not a studio parameter in phase 1. Twist duration and easing vary by family only within the table's motion range (150 to 900 ms; the easing family has no overshoot), a first proposal that G5's motion table (H-03) replaces.
- The worker keeps one learner per family and runs only the shown family's work; the limits of section 5.5 apply per family.

## 5. Model (theory in the owner's guide, sections 8.20 and 8.21)

### 5.1 Scale

The probit noise is fixed at σ = 1, which fixes the utility scale (the likelihood depends only on signal/σ). With the additive kernel of section 5.3 the signal amplitude is redundant with the length scales, so it is fixed at 1. The fitted hyperparameters are one length scale per parameter (0.05 to 50 on the normalized axes; both features of a circular parameter share it), ρ and the tie threshold. Each length has a log-normal prior density on its value (including the 1/l factor), with median e^√2 · √D, where D is the number of encoded features (about 18 for the default table's D = 20), and log-sd √3. A prior without the 1/l factor pulls every length to the median. The priors on ρ (logit-normal) and the tie threshold (log-normal, median 0.2) are unchanged.

### 5.2 Observations

With d = f(A) − f(B) and a tie threshold ε > 0 (fitted), the three outcomes are normalized:
- P(A over B) = Φ((d − ε)/√2),
- P(same) = Φ((ε − d)/√2) − Φ((−ε − d)/√2),
- P(B over A) = Φ((−d − ε)/√2).

"Both bad" adds no preference observation in phase 1. It marks both looks as rejected: candidates within a stated radius of a rejected look are excluded, and the next round explores away from the current best. An acceptability model is a later option. Undo removes the last record; the model is refitted from the remaining records, and the tests check that undo equals a fresh fit.

### 5.3 Fit

- Prior: zero-mean GP with the kernel of section 4.1, where k_x is additive over the parameters, one length scale l_p each. A linear or integer parameter contributes 2 s_p + s_p², with s_p = (x_p − ½)(x'_p − ½)/l_p² on the normalized axis; a circular parameter contributes 2 s_p, with s_p the dot product of the two looks' (cos, sin) features divided by l_p² (its first harmonic only). Each parameter thus adds a linear and a quadratic term of its own: a long length scale switches a parameter off, and the fitted lengths rank the parameters' relevance. (The first choice, ARD squared-exponential, explains every answer with one joint distance; one short irrelevant length then decorrelates all looks, and the relevance ranking failed in every simulated seed.)
- Weight-space form: f(x, s) = w · ψ(x, s) with w ~ N(0, I). Per parameter, two features: √2 (x_p − ½)/l_p and (x_p − ½)²/l_p², or √2 cos/l_p and √2 sin/l_p for a circular one. ψ holds them once scaled by √ρ (shared by all scenes) and once scaled by √(1 − ρ) in the block of the look's scene: 36 × 4 = 144 weights with the default table. This is exactly the kernel above, so a fit costs O(N·M² + M³) for N answers and M weights, independent of the number of looks.
- Update: Laplace approximation by Newton's method on the weights after each answer, starting at zero, at most 30 iterations; on non-convergence the previous fit is kept and the page says so.
- Hyperparameters: refitted every 10 answers by maximizing the approximate marginal likelihood plus the priors. Grid search: each length in turn over 13 log-spaced values from 0.1 to 50, then moves of ±0.8 and ±0.3 on logit ρ and log ε; two passes, at most 500 evaluations. Every grid value is evaluated directly, so the search cannot stall on the flat evidence of a long length.

### 5.4 Next pair

- Candidates per round, hard checks first: 512 from the feasible pool and local perturbations around the predicted optimum (the previous round's candidate with the highest posterior mean, shown or not); four single-parameter moves per parameter and 24 moves of two or three parameters from the predicted optimum; and the best shown look.
- The anchor is the candidate with the highest posterior mean. In half of the rounds its partner is the candidate with the largest mutual information between the three-outcome answer and the two latent utilities. In the other half the pair is the best, by the same measure, of the anchor pair and 3,000 random pairs of candidates. The mutual information is computed by 16-point quantile quadrature of the pair's posterior difference, without sampling. (The first choice, the best shown look against a Thompson draw, aimed at the current best rather than at the answers that fix the final pick and the relevance; pairs far from the optimum carry most of the relevance information.)

### 5.5 Limits

- At most 400 latent looks in total (all scenes). Pruning first removes candidates that were never shown. When the shown looks alone reach 400, the fit uses the comparisons among the 400 most recently shown looks; older comparisons stay stored and exported but leave the fit, and the page says how many.
- Work runs in a Web Worker. Target: the next pair is ready within 1 s at the maximum size. The benchmark in headless Chromium is indicative only, not a measurement on the owner's laptop. In the weight-space form the fit and the pair choice scale with the answers, the weights and the round's candidates, not with the looks; the look cap stays as a bound on stored state.

### 5.6 Settled

"Settled" when, in a scene: (1) the expected regret of the reported best look, E[max f − f(best)] over 200 joint posterior draws f of the round's candidates, divided by the predicted range D = (the highest posterior mean among the candidates) − (the median posterior mean over the scene's feasible pool), is at most τ. This is the normalization of the experiment's r with posterior means in place of the true utilities; D is fixed before the draws, so the statistic has a finite expectation (a draw-dependent denominator can approach zero and make it diverge; plan re-check finding TL-P11). The test fails when D ≤ 0, and the ratio is capped at 1 for display; (2) the best is unchanged for K answers in that scene; (3) two separate sessions end in the same region. First values τ = 0.05 and K = 15; the experiment calibrates them on development seeds so that criterion 4 of section 6 holds, and the report states the values used. (The first rule asked the best to beat every one of the round's 512 candidates in 90% of draws; near-identical candidates made that practically impossible, and it never fired in simulation. Owner decision of 3 October 2026.)

## 6. Simulated-owner experiment

- Synthetic utilities with a known optimum: a smooth function of three to five parameters, the others irrelevant, with scene-specific and shared parts; its optimum lies inside the feasible set of section 4.3.
- Answers use the observation model of section 5.2 at three noise levels (signal-to-noise 1, 2 and 4) and a fixed tie threshold.
- Measure normalized regret r = (u* − u(best)) / (u* − median u over the feasible sweep), not distance to one optimum vector.
- Run a low-dimensional baseline first, then the full space. At least 50 paired seeds per noise level; random pairs with the same budget as the baseline method.
- Pass, at the middle noise level, in sessions of 30 comparisons: at 90 comparisons, median r ≤ 0.10, the model beats random pairs on paired seeds (one-sided Wilcoxon signed-rank test, p < 0.01), and "settled" is wrong (declared while r > 0.20) in at most 10% of seeds; at 160 comparisons, the fitted length scales rank the relevant parameters above the irrelevant ones in at least 80% of seeds. The thresholds are those of 2 October 2026; the owner moved the evaluation points from 60 comparisons on 3 October 2026, after 60 answers proved too few to separate 4 relevant parameters from 14 irrelevant ones (a reference learner that knows the utility's form, centres and scale ranked them correctly in 58% of seeds). Until a family has 160 comparisons, the page marks its parameter ranking as provisional.
- In the celebrating scene the synthetic optimum lies within the scene limits (section 4.1).
- Two families (added by the owner design decisions; the criteria above are unchanged): the synthetic owner has two family optima, drawn so that their regions of regret at most 0.10 are disjoint: under the utilities' common weighted metric (circular for hue), the distance between the optima exceeds the sum of the two regions' radii, so no single look is within 0.10 of both optima. The answers of both families are stored together, interleaved, and each family's learner reads its own. A seed recovers both families when each family's reported best look has regret at most 0.10 against its own optimum, which takes two different looks. Pass: at the middle noise level and 90 comparisons per family (180 in total, three sessions of 30 per family), the median over seeds of the larger of the two families' regrets is at most 0.10. Before the run, the harness checks in every seed that the fixture's regions are disjoint and that a collapsed control (both families reporting the same look: either optimum, or the feasible look with the smallest summed regret) fails the predicate.
- The result is a sanitized report in `docs/progress/1.0/` and sets the expected number of comparisons before the owner starts.

## 7. Storage, privacy and export

- Capabilities: `db` (owner-only rules: every path readable and writable only at owner level), `user`, `downloads` for export. No `sample`, `mcp`, `room` or `comments`.
- Collections: `comparisons` (looks as parameter vectors, family, scene, answer, time, session, an optional reason), `sessions`, `space` (the edited parameter space, with the families and the scene limits), `model` (fitted hyperparameters).
- Claude reads the data with `ArtifactData` and writes a copy to `work/loop-memory/tastelab/` (private, ignored by Git). Export saves the same JSON to the owner's computer.
- Never stored: images, names, keys, anything from the owner's sessions.
- **Reason (optional).** After an answer, W opens a one-line field for that answer (at most 140 characters); Enter saves it into the answer's record and Esc closes the field. The reason stays in the page's owner-only storage like every answer and is exported with it. It is raw material for the owner's annotated references and nexus cards (4A), from which agents extract transferable attributes only, never assets.
- **Export.** The answers, plus `presets` (the best shown look per family and scene with its answer count and settled state: 2 × 3 starting points by default), `shared` (section 4.5) and `costHeavy`: gloss, glow, fog and edge weight, the parameters whose frame-time cost H-06 must measure before a preset reaches G3. Taste Lab measures no performance and the export makes no performance claim; each family's most expensive scene must pass the renderer gate in the design and engineering tracks.
- **Labels.** Every term the page shows follows the glossary (`docs/progress/1.0/glossary/glossary.md`): the motion parameter is the twist duration (glossary: twist), and the parameter that shrinks each drawn cell is the cell gap (a sticker slot is a different concept). Concepts the page names that the glossary does not define yet (colour class, scene, theme family) are listed in the report for the concept table of owner design decision 2; the page does not define them.

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

## 9. Deviations recorded during implementation

- The page entry is `tools/tastelab/index.html`, not `page/index.html`, so that the published file layout matches the repository: `page/worker.js` imports `../core/`.
- `page/candidates.js` was added. It builds the feasible pool once from 16,384 Sobol points (with the default table about 1.5% pass the hard checks, 247 looks) and fills each round from it after up to 256 local perturbations around the current best (σ = 0.08 on the unit axes, at most 1,024 tries).
- `page/learner.js` was added. It holds the learner state behind the worker, so that Node tests cover the look cap, the rebuild when the cap is reached during a session, skipped records and the settled rule. `page/worker.js` only passes messages.
- Settled (section 5.6): two best looks are in "the same region" when their scaled distance under the fitted length scales, the square root of Σ ((x_i − x'_i)/l_i)² over the encoded features, is at most 1. A session that settles stores its best look per scene in `sessions/<id>`. Stability counts answers per scene, not proposals or scene changes.
- Storage (section 7): the page uses `comparisons`, `sessions` and `space`. It does not store `model`; the worker refits the hyperparameters from the comparisons on every load.
- A stored parameter table is used only when it is valid and names every parameter the page draws; hard checks use the session's table. Records that do not fit the current table are skipped, and the page shows how many.
- Hyperparameters (sections 5.3 and 5.5): the worker runs the search of every tenth answer, and the one after a load, in its idle time, one evaluation per task, and proposes the next pair first. Until the search ends, pairs use the previous hyperparameters. A search still running when the next one starts hands over its best point; answers that arrive during a search are fitted when its result is adopted.
- Next pair (section 5.4): one joint posterior of the round's candidates serves the pair choice and the settled test; the best look and the pool's median need only posterior means.
- Settled (section 5.6): the regret test concerns the reported best look, the shown look with the highest posterior mean, which every round includes.
- Experiment (section 6), owner decision of 2 October 2026 to fix these as defects: the noise level is the latent signal over σ, d = level · Δu / sd(u) over the feasible sweep; random pairs are two distinct looks drawn uniformly from the feasible sweep; u* is the scene's optimum, which passes the hard checks; the median and the spread come from the setting's own feasible sweep. Both methods run the page's learner (`page/learner.js`) on their own answers. Sessions are 30 comparisons long (three by the 90-comparison checkpoint), so that "settled" is scored by the page's two-session rule; one session's settled state is reported separately.
