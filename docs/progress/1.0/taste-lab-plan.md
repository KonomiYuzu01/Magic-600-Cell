# Taste Lab phase 1: plan

Status: **draft plan, before its Codex plan check.** Owner decision of 2 October 2026 ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)): Taste Lab is built now, its front end is a private Artifact page on claude.ai, and its model learns from few pairwise choices and draws new variants itself.

## 1. Goal and acceptance

Give the owner a private page on which a model learns the owner's visual preferences for the 600-cell from a few dozen pairwise choices, and hands the result to the design track (G2 to G5) as starting points.

Phase 1 is accepted when:
1. the core tests pass headless (`python tests/test_tastelab_core.py`);
2. a simulated-owner experiment (section 6) reports how many comparisons the model needs to find a known optimum, over many seeds, with the noise level stated;
3. the page is published privately, its storage rules allow writes by the owner only, and one functional pass (record a comparison, read it back with `ArtifactData`) succeeds;
4. no class B image, personal data, key or private path is in the page, its storage or the repository.

## 2. Scope

- **Phase 1a (this plan):** the parameter studio. No downloads, no installs, no external sources: the page draws every look itself.
- **Phase 1b (later, one owner approval first):** the class A image swipe. It needs an embedding model (about 1 GB of weights) and a deep-learning runtime on the allowlist, plus the class A source domains. These are "Ask the owner" items (allowlist sources, large model download), even in the cloud container. The page reserves a tab for it.
- **Out of scope:** class B images (phase 2, local only); sound; shipping a look tuner to end users (a 1.0 candidate for stage 2.2); the full-detail renderer (Look Lab, G3).

## 3. Parts

| Part | Path | Owner | Notes |
|---|---|---|---|
| Colour core | `tools/tastelab/core/color.js` | Codex packet P1 | sRGB and OKLab conversion, ΔE in OKLab, colour-vision-deficiency simulation (protan, deutan, tritan; Machado 2009 matrices) |
| Preference model | `tools/tastelab/core/gp.js` | Codex packet P1 | Gaussian-process preference model with probit likelihood, Laplace approximation, ARD squared-exponential kernel, marginal-likelihood fit of length scales, signal and noise |
| Next-pair choice | `tools/tastelab/core/acquire.js` | Codex packet P1 | Sobol candidates plus local perturbations around the current best, hard checks first, current best as one side, Thompson sampling for the other, about 20% information-gain pairs, repeat pairs for consistency |
| Parameter space and checks | `tools/tastelab/core/space.js` | Codex packet P1 | parameters, ranges, scenes, hard checks |
| Core tests | `tools/tastelab/core/tests/*.test.mjs`, `tests/test_tastelab_core.py` | Codex packet P1 | `node --test` run from the Python test; the Python test skips with a clear message when Node.js is missing |
| Geometry | `tools/tastelab/page/geometry.js` (+ test) | Codex packet P2 | 120 vertices of the 600-cell as unit quaternions, 720 edges, 1,200 triangles, 600 tetrahedral cells, cell adjacency, the 20 rings of 30 cells; tested against these counts |
| Preview | `tools/tastelab/page/preview.js` | Claude | WebGL 2: perspective projection from 4D, cells shrunk by the gap parameter, edges, gloss, glow, fog; one animated cap turn with the chosen duration and easing |
| Page | `tools/tastelab/page/index.html`, `app.js` | Claude | two looks side by side, keys A, B, S (same), X (both bad), Z (undo), "what matters" panel, scene switch, export |

The page publishes its JavaScript as supporting files of the Artifact, so no bundler is needed. The repository holds only code; the page's storage holds the owner's data.

## 4. Parameter space (first proposal; the owner edits it before the first session)

| Group | Parameter | Range |
|---|---|---|
| Palette | hue rotation | 0 to 360 degrees |
| Palette | hue spread | 60 to 360 degrees |
| Palette | lightness (OKLab L) | 0.45 to 0.85 |
| Palette | chroma (OKLab C) | 0.04 to 0.20 |
| Background | lightness, tint hue, tint strength | 0.05 to 0.95; 0 to 360; 0 to 0.05 |
| Geometry | sticker gap, edge weight, edge brightness | 0 to 0.3; 0 to 3 px; 0 to 1 |
| Material | gloss, glow, fog density | 0 to 1 each |
| Motion | turn duration; easing shape (two numbers of one curve family) | 150 to 900 ms; 0 to 1 each |

- **Scenes:** solving, inspecting, celebrating. The scene enters the model as a context input with its own length scale, so looks can differ by scene where the owner's choices say so and stay shared where they do not.
- **Colouring rule:** cells are coloured by a structural rule; the first version colours the 20 rings of 30 cells. The real sticker palette is G4's job; the studio learns the palette character (hue spread, lightness, chroma, background).
- **Hard checks before a look is shown:** touching cells of different colour classes differ by at least the ΔE threshold in OKLab under normal vision and under each simulated deficiency (threshold proposed in G4; first value 0.08); background contrast; parameter ranges. A failing look is never shown.

## 5. Model (theory in the owner's guide, sections 8.20 and 8.21)

- Choice likelihood: P(A over B) = Φ((f(A) − f(B)) / (√2 σ)). "Same" is a tie with a learned threshold; "both bad" records both below a virtual reference at the level of the chosen looks; undo removes the last record.
- Prior: zero-mean GP with an ARD squared-exponential kernel over normalized parameters plus the scene context.
- Update: Laplace approximation by Newton's method after each answer; hyperparameters re-fitted every 10 answers by maximizing the approximate marginal likelihood.
- Next pair: current best against a Thompson sample among the top candidates by an upper confidence bound; about 20% of pairs by expected information gain; about 5% repeats of earlier pairs.
- Size: at most a few hundred looks per scene; older looks far from every choice may be pruned.
- "Settled" when the model is at least 90% sure that the best beats every candidate of its round, the best is unchanged for 15 answers, and two separate sessions end in the same region.

## 6. Simulated-owner experiment

A synthetic utility with a known optimum (a smooth function of a few parameters, the others irrelevant) answers the page's questions with probit noise of several sizes. Over at least 50 seeds per noise level, record the distance from the found best to the true optimum against the number of comparisons, whether the length scales single out the relevant parameters, and how often "settled" is wrong. Compare with random pairs. The result is a sanitized report in `docs/progress/1.0/` and sets the expected number of comparisons honestly before the owner starts.

## 7. Storage, privacy and export

- Capabilities: `db` (owner-only rules: every path readable and writable only at owner level), `user`, `downloads` for export. No `sample`, `mcp`, `room` or `comments`.
- Collections: `comparisons` (looks as parameter vectors, scene, answer, time, session), `sessions`, `space` (the edited parameter space), `model` (fitted hyperparameters per scene).
- Claude reads the data with `ArtifactData` and writes a copy to `work/loop-memory/tastelab/` (private, ignored by Git). Export saves the same JSON to the owner's computer.
- Never stored: images, names, keys, anything from the owner's sessions.

## 8. Process

1. Codex plan check of this plan (Sol, max, standard tier; no critical path is touched).
2. Commit the interfaces and test skeletons, then run packets P1 and P2 through `--kind implement` in parallel while Claude writes the preview and the page.
3. Review: Claude reviews Codex's patches; a Codex review covers Claude's page code; one review of the finished candidate.
4. Checks: `python tests/test_tastelab_core.py`, the simulated-owner experiment, a browser check of the page with the pre-installed Chromium (rendering and keys), then publish and the functional pass.
5. Owner: edit the parameter space, then start comparing.
