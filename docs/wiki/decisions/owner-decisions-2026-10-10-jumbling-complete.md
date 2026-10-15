---
id: owner-decisions-2026-10-10-jumbling-complete
type: decision
status: verified
visibility: public
summary: Owner decision of 10 October 2026 - the jumbling restoration method must restore every reachable configuration completely; a guarantee limited to shallow scrambles (for example 20 twists) is not acceptable. Also asks for detailed observation requirements (piece filters, raw view).
related: [owner-decisions-2026-10-10-jumbling-deferred, owner-decisions-2026-10-09-jumbling]
supersedes: []
claims:
  - {id: complete-restoration-required, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: observation-requirements-requested, evidence_kind: decision, checked_at: 2026-10-10}
---

# Owner decision, 10 October 2026: complete restoration of every jumble

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Context

Phase U of the [restoration method](../../../research/jumbling/theory/restoration.md) reaches the lattice on every tested input of up to 20 twists. No 40- or 60-record run has reached it yet; the nearest, I_a@40, stops at N = 1,192 (open problem R1 in [open-problems](../../../research/jumbling/theory/open-problems.md)). The integrator offered two options if the current joint attack did not get past that stop: keep researching words that raise N before they lower it, or let 1.0 promise restoration only up to 20 twists and leave deep restoration to later research.

## Decision

1. **A shallow guarantee is not acceptable.** The 20-twist option is rejected. The bottom line is a complete solution: a method that restores any reachable jumbled configuration, at any depth.
2. **Observation requirements.** The owner asked for a detailed requirements page on how a solver observes a jumbled state (piece filters, the raw puzzle), to go in together with the current round of experiments: [jumbling-observation](../../progress/1.0/jumbling-observation.md).

## Effect

- Open problem R1 is the acceptance of the restoration work: it closes only with a method that is complete for every reachable configuration, not with a depth limit.
- The owner's earlier decision stands: the method must be human-executable and read only the visible configuration, within the human-solve boundary ([owner-decisions-2026-10-10-jumbling-deferred](owner-decisions-2026-10-10-jumbling-deferred.md)).
- Limiting scramble depth is not an option at the twist menu sign-off of the 2.5 freeze ([owner-decisions-2026-10-09-jumbling](owner-decisions-2026-10-09-jumbling.md)).
