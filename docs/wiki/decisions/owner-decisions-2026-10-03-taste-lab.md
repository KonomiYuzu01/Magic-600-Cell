---
id: owner-decisions-2026-10-03-taste-lab
type: decision
status: verified
visibility: public
summary: Owner decisions of 3 October 2026 on the Taste Lab learner - the pass thresholds stay, criteria 1, 2 and 4 and the two-family criterion are checked at 90 comparisons per family and the relevance ranking at 160; the settled rule is recalibrated.
related: [owner-decisions-2026-10-02, owner-decisions-2026-10-02-design]
supersedes: []
claims:
  - {id: evaluation-points, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: provisional-ranking, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: settled-recalibrated, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: plan, evidence_kind: source, path: docs/progress/1.0/taste-lab-plan.md, checked_at: 2026-10-03}
---

# Owner decisions, 3 October 2026: Taste Lab learner

The simulated-owner experiment of the [Taste Lab plan](../../progress/1.0/taste-lab-plan.md) (section 6) did not meet its pass criteria at 60 comparisons, so by the plan's section 1 item 3 the result went to the owner before the learner is offered as one. The owner's reply was in Chinese; this page records it in English. The thresholds themselves were not changed by anyone but the owner.

## Result that went to the owner

Development seeds only, synthetic owner, middle noise level, a redesigned learner from an escalated diagnosis (additive kernel with one length per parameter, a grid search for the lengths, pairs chosen by expected information). The numbers come from a scratch harness and are re-measured on the repository harness before acceptance; the acceptance seeds have not been run.

- Criterion 1 (median regret at most 0.10): 0.130 and 0.092 on two 50-seed blocks at 60 comparisons, at the threshold; 0.055 on both at 90.
- Criterion 2 (beats random pairs, Wilcoxon p below 0.01): passed on both blocks at 60, one of them narrowly; p about 1.5e-5 at 90.
- Criterion 3 (relevant parameters ranked above the irrelevant ones in at least 80% of seeds): 40% and 20% at 60, about 60% at 120, 80% from 160 on the untouched block. Sixty three-outcome answers carry too little information to separate 4 relevant parameters from 14 irrelevant ones: a reference learner that knows the utility's form, centres and scale ranks them correctly in 58% of seeds with 60 random pairs.
- Criterion 4 (false "settled" in at most 10% of seeds): 0%, but only because the settled rule never fired in any run.

## Decisions

1. **Thresholds unchanged, later evaluation points.** Criteria 1, 2 and 4 and the two-family criterion are checked at 90 comparisons per family (two families: 180 in total, three sessions of 30 per family). Criterion 3 is checked at 160 comparisons per family. Before a family reaches 160 comparisons, the page marks its parameter ranking as provisional. The experiment report states these numbers as the expected comparison counts before the owner starts (plan section 6).
2. **Settled rule recalibrated.** The rule in plan section 5.6 asked the best look to beat every one of the round's 512 candidates in 90% of posterior draws, which near-identical candidates make practically impossible. It is replaced by a test on the expected regret of the best look under the posterior, so that it can fire when the recommendation is stable. Criterion 4 is unchanged and decides the calibration in simulation.

The learner redesign itself is an engineering change recorded in the plan and reviewed under the normal rules for non-critical code.
