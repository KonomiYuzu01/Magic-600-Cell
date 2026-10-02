---
id: owner-decisions-2026-10-02-scope
type: decision
status: verified
visibility: public
summary: Owner decisions of 2 October 2026 - only sound and the mathematical-language work are reduced in scale (design stays the core of 1.0), review effort is tiered by risk with a weekly process budget, and the source of G1 to G6 testers is open.
related: [owner-decisions-2026-10-02-design, owner-decisions-2026-09-30]
supersedes: []
claims:
  - {id: math-language-reduced, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: sound-reduced, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: design-not-reduced, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: risk-tiers, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: process-budget, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: rules-text, evidence_kind: source, path: AGENTS.md, checked_at: 2026-10-02}
---

# Owner decisions, 2 October 2026: scope and process

After a project review named five risks (the unmeasured renderer gate, the owner as the only Windows executor, a large design scope, heavy process overhead and the open engine choice), the owner decided as below. The owner's reply was in Chinese; this page records it in English. The renderer, Windows-batch and engine items need no rule change; they are carried out as planned in [renderer-candidates](renderer-candidates.md) and the stage 2.4 plan.

## 1. Scope: only sound and the mathematical language are reduced

The owner: reduce the scale of sound and of the mathematical-terminology work; everything else stays; design is the core of this update. This amends gaps 2 and 10 of [owner-decisions-2026-10-02-design](owner-decisions-2026-10-02-design.md); it replaces neither.

**Mathematical language** (amends gap 2, R2b; the relationship strip and the encoding stay as decided):
- The 1.0 theory book covers only the terms, symbols and relations that the 1.0 UI actually shows: the relationship strip, the encoding and the term cards. Everything else moves to 1.x.
- Each such term keeps a definition and a term card with a diagram rendered from the real geometry (M4 and M6 otherwise unchanged).
- Proofs are required only for facts that the UI's correctness relies on, for example that a proper colouring of the rings with k classes exists, or that a buffer position differs from its occupant. A short proof or a cited standard proof is preferred. Background mathematics gets no written-out proofs in 1.0.
- The mathematics review (M5) is limited to that set.

**Sound** (amends gap 10, 10B):
- The G5 audition is one short comparison in a single session: silent against a two-cue set.
- If the owner adopts sound afterwards, 1.0 has at most two cues ("completed" and "invalid action"), each with a visual equivalent and a mute. Sound semantics in the motion table and the full cue set move to 1.x.
- If sound is not adopted, 1.0 is silent and stage 2 does no further sound work.

**Not reduced:** two theme families with three scene looks each; the relationship strip; rewards; the two linked views; one entry with coaching and the practice copy; text budgets; G1 to G6 except the sound part of G5.

## 2. Process: risk tiers and a process budget

The owner approved the following text verbatim; it is in `AGENTS.md` under "Review rounds":

- Critical paths keep every existing review rule.
- Other code and tools: no plan check unless the change alters behaviour or a contract; one Sol review with `--speed fast` of the finished candidate; no verification round for findings below `major`.
- Documentation, wiki and progress pages that record owner decisions or measured results: no Codex review; the wiki and agent-rules checks run.
- Process budget: in each working week at least 70% of the commits are experiment code, measurements or product content, and at most 30% are governance or documentation. The integrator reports the split in the milestone summary. When governance exceeds 30%, new governance work stops until the split recovers, except fixes to a broken check, hook or wrapper.

This page and the rule change were themselves recorded under the owner-dictated exemption: no Codex review, agent-rules and wiki checks only.

## 3. Tester source for G1 to G6 (open; owner decides)

The trial numbers of the design decisions (text budgets, motion timing, encoding legibility) are set by the stage 2.3 tests G1 to G6. Who takes them is open:

| Option | Pros | Costs |
|---|---|---|
| (a) The owner only | No recruiting, no data handling, fastest; the owner is a core user | One person's habits set every number; no check of learnability for newcomers |
| (b) 3 to 5 recruited puzzlers from the hypercube community | Real core and secondary users; also an early, private first showing | Recruiting time; consent and data rules for their sessions; needs a build worth showing |
| (c) Trial values now, tuning in 1.x | No test cost in stage 2 | Numbers stay guesses through 1.0; the risk moves to users after release |

Not decided here. Options can combine, for example (a) in stage 2 and (b) on the vertical slice.
