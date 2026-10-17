---
id: owner-decisions-2026-10-17-jumbling-m2
type: decision
status: verified
visibility: public
summary: Owner decision of 17 October 2026 - gap M2 of the jumbling restoration theory is closed, and help that lets a person carry out the lattice phase moves to the engineering line; the proofs will be checked by a strict Lean formalization once gap M1 is solved, from a complete theory handoff document; gap M1 is next.
related: [owner-decisions-2026-10-15-jumbling-restored, owner-decisions-2026-10-16-x1-api-budget]
supersedes: []
claims:
  - {id: m2-closed-for-the-theory, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: lean-formalization-after-m1, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: m1-next, evidence_kind: decision, checked_at: 2026-10-17}
---

# Owner decision, 17 October 2026: gap M2 closed, proofs to a Lean formalization

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Decisions

1. **Gap M2 is closed for the theory.** M2 asked whether the length of the lattice phase is within reach of a person ([METHOD.md](../../../research/jumbling/restoration/09-general-method/METHOD.md) section 5). The proved bounds stand: at most 20,866,844 twists for every input, or at most 4,655,642 actions (single twists and applications of 70 stored macros). The owner will hand to the engineering line the help that lets a person carry out the lattice phase with the program's assistance.
2. **The proofs go to a Lean formalization.** The owner has set up a strict Lean formal-verification agent (Archon) in the owner's Codex workspace. Once gap M1 is solved, the integrator writes a complete handoff document of the theory for it. Gap E2, the review of the proofs, is to be closed this way instead of by a human reviewer.
3. **Gap M1 is next.** After the pull request that imports the X1 result, the research works on M1: a construction from the state alone with a bound on its length. Since R(I_a) is infinite, that bound must depend on the state.

## Effect

- **Pages.** The restoration pages mark M2 closed and record the plan for E2. M1 stays the open core gap.
- **Unchanged.** Three things stay as they were:
  - the definition of restored and the research priority of 15 October 2026;
  - the product's human-solve boundary of 2 October 2026;
  - the rule that other project work runs on subscription quota.
