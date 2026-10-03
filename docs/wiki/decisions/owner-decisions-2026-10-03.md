---
id: owner-decisions-2026-10-03
type: decision
status: verified
visibility: public
summary: Owner decisions of 3 October 2026 - the owner alone takes the G1 to G6 tests in stage 2, the stage 2.0 charter is signed as written, and a Windows continuous-integration job for headless checks is added.
related: [owner-decisions-2026-10-02-scope, owner-decisions-2026-10-02-design, owner-decisions-2026-10-03-migration]
supersedes: []
claims:
  - {id: tester-source-owner, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: charter-signed, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: windows-ci, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: charter-text, evidence_kind: source, path: docs/progress/1.0/charter-2.0-draft.md, checked_at: 2026-10-03}
---

# Owner decisions, 3 October 2026: testers, charter and Windows CI

The owner answered three open items in one chat message, in Chinese. This page records the answers in English.

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
