---
id: owner-decisions-2026-10-03
type: decision
status: verified
visibility: public
summary: Owner decisions of 3 October 2026 - the owner alone takes the G1 to G6 tests in stage 2, the stage 2.0 charter is signed as written, a Windows continuous-integration job for headless checks is added, a construction the solver selects by family and parameters counts as the solver's own macro, and the owner signs all stage 2.2 delete and automate dispositions.
related: [owner-decisions-2026-10-02-scope, owner-decisions-2026-10-02-design, owner-decisions-2026-10-03-migration]
supersedes: []
claims:
  - {id: tester-source-owner, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: charter-signed, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: windows-ci, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: selected-construction-is-own-macro, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: dispositions-signed, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: charter-text, evidence_kind: source, path: docs/progress/1.0/charter-2.0-draft.md, checked_at: 2026-10-03}
---

# Owner decisions, 3 October 2026: testers, charter, Windows CI, selected constructions and dispositions

The owner answered three open items in one chat message, and two stage 2.2 questions in later messages, all in Chinese. This page records the answers in English.

## 1. Tester source for G1 to G6: option (a), the owner only

This settles section 3 of [owner-decisions-2026-10-02-scope](owner-decisions-2026-10-02-scope.md).

- The owner alone takes the stage 2.3 tests G1 to G6 (text budgets, motion timing, encoding legibility and the rest). The trial numbers are set from the owner's results.
- No testers are recruited in stage 2. No consent or data-handling rules for other people's sessions are needed.
- The known cost stays recorded: one person's habits set every number, and learnability for newcomers is not checked in stage 2. A later round with recruited puzzlers, option (b), is not excluded. It needs a new owner decision.

## 2. Stage 2.0 charter: signed as written

- The owner approved the [charter](../../progress/1.0/charter-2.0-draft.md) as written. The signature is the exit of stage 2.0.
- The signature confirms these as written:
  - section 4 (scope of 1.0);
  - the wording of section 5 (design charter);
  - the reasons Claude gave for the anti-goals;
  - the quality bar.
- What the signature does not settle:
  - the manifesto (stage 2.3);
  - the review-lens role cards (a later process change);
  - every number marked _proposal_ (set in 2.3, frozen at 2.5).
- The owner may still add anti-goals.

## 3. Windows continuous integration: yes

- A GitHub Actions job on a GitHub-hosted Windows runner runs the repository's headless checks on every pull request and on `main`. The repository is public, so the hosted runners add no cost.
- **Evidence class.** A pass is evidence for "headless checks pass on Windows", nothing more.
  - It is not actual Windows/DirectX evidence.
  - It is not input, long-session or performance evidence.
  - The native host regression, the renderer gate and the migration probes still run on the owner's machine.
- The workflow is non-critical tooling under the [risk tiers](owner-decisions-2026-10-02-scope.md): one Sol review with `--speed fast` of the finished candidate.

## 4. A construction the solver selects is the solver's own macro

Asked during the stage 2.2 dispositions ([dispositions README](../../progress/1.0/dispositions/README.md), "Integrator check").

- **Ruling.** The solver may select a known construction by family and parameters: a placement star, an orientation transfer, a buffer-A correction or the final-buffer commutator. Such a selection counts as the solver's own macro, not a macro the program chose. The program expands it into its exact word, shows the complete effect and the protection checks, and executes it only on the solver's explicit action.
- **Consequence for 2.2.** The seven drafted deletions of these constructors become `redesign`: U2-013, U2-015, U2-017, U2-019, U3-093, U3-095 and U6-033/2.
- **Unchanged.** The [human-solve boundary](owner-decisions-2026-10-02-design.md) still holds. The program never picks a family, parameter, target or correction by itself. The insertion planner (`/api/suggest`) chooses these from the orbit state, so it stays a proposed `delete` for the owner's signature.

## 5. Stage 2.2 dispositions: all delete and automate items signed

- The owner signed all 18 decisions in the "For the owner's signature" table of [dispositions.md](../../progress/1.0/dispositions/dispositions.md): 12 `delete` and 6 `automate`, after the ruling in section 4. This is the owner part of the stage 2.2 exit (charter section 8).
  - `delete`: U1-007, U1-063, U2-012, U2-022, U3-002, U4-038/2, U4-069/4, U4-082, U4-083, U4-084, U4-086, U4-087.
  - `automate`: U2-063, U3-085, U3-104, U3-119, U4-050/2, U6-037/2.
- The flows the dispositions use are still provisional. When the owner's core-flow storyboards change a flow, only the decisions that use it are reopened; a reopened `delete` or `automate` needs a new signature.
