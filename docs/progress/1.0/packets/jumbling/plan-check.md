# Packet: plan check of the jumbling program (senior reviewer, plan kind)

Run read-only as a plan check by the senior Codex reviewer, with effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: decide whether the program plan `docs/progress/1.0/jumbling-plan.md` and its four amendments to the plan-checked state contract (plan section 4, A1–A4) are correct and sufficient before the contract is amended, the theory work (J3) starts and the renderer packet is drafted. Owner decisions (9 October 2026, `docs/wiki/decisions/owner-decisions-2026-10-09-jumbling.md`):
  - jumbling enters 1.0 as an independent puzzle and a Jumble feature;
  - the simulator, rendering and observation prototype, solving theory and grip-orbit explorer start now;
  - a renderer packet follows at mid-stage.
- Acceptance: a schema-valid result in which each question Q1–Q8 below has a finding or an explicit statement that it holds. Evidence is `path:line`, or a counterexample with an adjudicating experiment where the reviewer disagrees.
- Out of scope:
  - the owner decisions themselves;
  - the choice of the twist menu (owner sign-off at the 2.5 freeze);
  - the contract sections already accepted;
  - any performance, Windows or DirectX claim;
  - the code of J1, J2 and J4, which gets its own review as a finished candidate.

## 2. Actual problem and reproduction
- The plan and the questions:
  - **Q1 (A1, Cayley twists).** Is the Cayley parametrisation over Q(√5)³ in the frame {n_c i, n_c j, n_c k} exact and sufficient for a general simulator? This frame does not commute with K⁺ transport in general. Should menus be transported by K⁺ conjugation from cap 0 instead, and is that well-defined modulo A4?
  - **Q2 (A2, filtered sign test).** Is a float evaluation of h_A at an exact vertex, with a proven forward error bound and an exact fallback, an acceptable certificate under contract section 3? What must the certificate record?
  - **Q3 (A3, stickers and frames).** Is "a sticker is a (piece, host facet) pair carried by the pose; on lattice states it agrees with the retained slot permutation and frames" sufficient for labelled-state agreement and for the renderer data contract?
  - **Q4 (A4, puzzle definition).** Is J(Λ), a per-cap menu containing A4 and transported by K⁺, a sound definition of the independent puzzle? Which candidate menus deserve the owner's attention? Item 3 of `research/jumbling/theory-problem.md` notes that any infinite-order element makes R(Λ) infinite.
  - **Q5 (J1 acceptance).** Are the five acceptance items in plan section 1 sufficient for the reference engine? The proposed route for agreement on all 1,200 generators is combinatorial: perm_k(M(src)) = M(dst), plus exact K⁺ transport of regions computed once per orbit representative. Is that a proof, or does it need a geometric check per generator?
  - **Q6 (renderer packet).** Does the outline in plan section 3 cover what the renderer route needs? It includes the per-piece transform buffer, the lattice flag, the overlays, workload W-J and the re-acceptance rule (lattice-state gate evidence stays valid; the jumbling mode gets its own W-J runs; a geometry change re-runs all acceptance).
  - **Q7 (J3 scope).** Is `research/jumbling/theory-problem.md` the right problem? Item 4 asks whether jumbling reaches lattice states outside the retained group G; is that the decisive question for the solving theory? Is any item ill-posed or missing?
  - **Q8 (order).** J1, J2 and J4 started under the existing contract before this check. Is there a reason to stop or reorder any of them? Are the mid-stage criteria (plan section 2) right?

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head. Evidence kind: source, fixture and synthetic geometry. Linux cloud session; no Windows, Direct3D 12 or performance evidence exists for jumbling.

## 4. Necessary source and evidence
- `docs/progress/1.0/jumbling-plan.md`, all sections.
- `research/jumbling/state-contract.md`, the plan-checked contract.
- `research/jumbling/README.md`, the exact witness results and review dispositions.
- `research/jumbling/theory-problem.md`.
- `research/jumbling/exact.py` and `witness.py`, the existing exact code.
- `docs/progress/1.0/renderer-experiment-plan.md` sections 1–2 and `docs/progress/1.0/packets/renderer/README.md`, for the renderer route and its gate.
- `research/theory/RETHLAS_BLUEPRINT.md`, for the retained theory and hypotheses R, F, I, C and E.

## 5. Attempts so far

| # | Step | Result |
|---|---|---|
| 1 | Plan check of the study and two scoped re-checks | Passed; all findings adopted (`research/jumbling/README.md`) |
| 2 | Exact witness E0–E4 and its code review | Done; three minor findings adopted |
| 3 | Owner decisions of 9 October 2026 | Option B; this plan |

## 6. Constraints and owned files
- Read-only review; no files may change.
- `assets/` and every critical path stay unchanged. 600-cell-Full is unchanged.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with finding IDs prefixed `J` (for example `J-Q1`).
- Review only; do not perform follow-up work.
