---
id: owner-decisions-2026-10-03-taste-lab
type: decision
status: verified
visibility: public
summary: Owner decisions of 3 October 2026 on the Taste Lab learner - the pass thresholds stay, criteria 1, 2 and 4 and the two-family criterion are checked at 90 comparisons per family and the relevance ranking at 160; the settled rule is recalibrated; after the cross diagnosis the current learner is final and the acceptance runs once; after the acceptance the learner is accepted with recorded limitations, and the two-family checkpoint moves to 150 comparisons per family with one confirmation run on fresh seeds.
related: [owner-decisions-2026-10-02, owner-decisions-2026-10-02-design]
supersedes: []
claims:
  - {id: evaluation-points, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: provisional-ranking, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: settled-recalibrated, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: final-learner, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: acceptance-ruling, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: two-family-checkpoint, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: known-limitations, evidence_kind: decision, checked_at: 2026-10-03}
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

## Later on 3 October: the cross owner and the final learner

With the calibrated learner, the cross robustness owner missed two criteria on development seeds: median regret 0.123 and 0.107 at 90 comparisons on two 50-seed blocks, and the strict relevance ranking at 160 comparisons in 74% and 70% of seeds. A time-boxed diagnosis, with a plan check and a limit of two variants, found no change that could ship. Other fits of the same answers did not lower the regret at 90, and candidate moves that vary two short-length parameters together left cross and quad unchanged. By 90 comparisons a look with regret at most 0.10 had been shown in 47 of 50 seeds, but the posterior ranked it below the reported best. The diagnosis is in the history section of the [experiment report](../../progress/1.0/taste-lab-experiment.md). The acceptance seeds had not been used.

3. **Current learner final.** The owner made the current learner final for phase 1a and had the acceptance run once on seeds 0 to 49. This was the recommended option; the alternative was at least a day more work on the pair choice with an uncertain outcome. The thresholds stay. The owner rules on the cross result of the acceptance run.

## Later on 3 October: the acceptance ruling

The acceptance ran once on seeds 0 to 49 with the final learner (code digest `7eb0b7fca8018935`); no threshold and no code changed. Results at the middle noise level (details in the [experiment report](../../progress/1.0/taste-lab-experiment.md)):

- Full setting, one family: all four criteria pass (median regret 0.070 at 90 comparisons; Wilcoxon p 0.0005 against random pairs; no false "settled" state by 90; strict ranking at 160 in 82% of seeds).
- Bump owner: all four pass (the ranking at 160 in 80% of seeds, at the threshold).
- Cross owner: the strict ranking at 160 holds in 58% of seeds; the other three criteria pass (median regret 0.075 at 90).
- Two families: the median of the larger regret at 90 comparisons per family is 0.123 against 0.10. Each family alone reaches regret at most 0.10 in 68% and 72% of seeds, as one family does alone; both together in 46%.
- Low setting: the median regret passes (0.093); the comparison with random pairs (p 0.76), false "settled" states (18% of seeds by 90) and the ranking (62%) miss. The fixed parameters carry no data, so the ranking cannot separate them in this setting (already noted on 2 October), and the settled rule was calibrated in the full setting only. The page has no mode that varies only some parameters.
- The low setting and the two families had not been run on development seeds with the final learner before the acceptance; the integrator reported this to the owner.

4. **Learner accepted with recorded limitations (owner exception).** The owner chose the recommended option of three. The others were to accept without a confirmation run, or to fix the low setting and the ranking first and run all five runs again on fresh seeds (at least half a day, with an uncertain outcome).
   - Two families: the threshold of 0.10 stays, and the checkpoint moves from 90 to 150 comparisons per family; this replaces the two-family part of decision 1. One confirmation run on the unused seeds 200 to 249 (full setting, quad owner, two families, middle noise, sessions of 30, 180 comparisons per family, the same code) is judged at 150 comparisons per family: the median over seeds of the larger of the two families' regrets is at most 0.10. The report also states the values at 90, 120 and 180. This rule was fixed before the run. If the harness stops on a fixture check, or the run misses, the result goes to the owner again; no seed is replaced. Result: it passes. The median of the larger regret is 0.045 at 150 comparisons per family, with both families within 0.10 in 45 of 50 seeds (0.106 at 90 and 0.084 at 120 comparisons per family).
   - Low setting: a known limitation. Before any page mode narrows the parameter space, such as a parameter-space editor, the settled rule is calibrated again in the narrowed space, and the ranking check is defined for parameters that never vary.
   - Cross owner: a known limitation. With interacting preferences the parameter ranking is not reliable at 160 comparisons. The report states that the ranking panel is a hint; the presets and the export do not depend on it.
