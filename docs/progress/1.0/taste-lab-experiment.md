# Taste Lab phase 1: simulated-owner experiment

Status: **accepted with recorded limitations, by owner decision** (3 October 2026). The learner of [plan section 5](taste-lab-plan.md#5-model-theory-in-the-owners-guide-sections-820-and-821) was run on the acceptance seeds 0 to 49 once, with the code that the page ships, against the criteria of [plan section 6](taste-lab-plan.md#6-simulated-owner-experiment) as the owner set them on 2 and 3 October 2026 ([owner-decisions-2026-10-03-taste-lab](../../wiki/decisions/owner-decisions-2026-10-03-taste-lab.md)). No threshold was changed. In the full setting with one family and with the bump owner every criterion passes; the cross owner misses the relevance ranking at 160 comparisons, two families miss at 90 comparisons per family, and the low setting misses three criteria. The owner accepted the learner with these as recorded limitations and moved the two-family checkpoint to 150 comparisons per family, where one confirmation on fresh seeds passes: the median of the larger of the two regrets is 0.045 (section 3.3).

Evidence class: synthetic simulation only, run headless in Node.js 24.21.0 on Windows. No owner answers, sessions or images were used. The latency figures in section 6 are indicative only: headless Chrome on the development machine, not the owner's laptop.

## 1. Setup

- Learner: the page's own module (`tools/tastelab/page/learner.js`) with its defaults: one length scale per parameter, a kernel with a quadratic part (for the circular hue rotation the second harmonic) and evidence-weighted smooth and interaction parts, a fit on the latent utilities of the shown looks, a grid search of the hyperparameters every 10 answers with refinement steps between the grid values (at most 600 evaluations), pairs from an anchor at the highest posterior mean and a partner by expected information, and the settled rule with τ = 0.05 and K = 10 (section 2).
- Feasible sweep: 247 of 16,384 Sobol looks pass the hard checks with the default parameter table.
- Synthetic owner "quad": u = −Σ w·(x − c)² over four relevant parameters on their unit axes: hue rotation (circular, w = 1), chroma (1.5), cell gap (1) and glow (1). The optimum c is a feasible look drawn per seed; in the celebrating scene the preferred glow moves by 0.35. The other 14 parameters are irrelevant. Each seed uses one scene.
- Robustness owners: "cross" adds −2 · 0.6 · √(1.5) · d_chroma · d_gap (an interaction that keeps the optimum); "bump" replaces each term by w · (1 − exp(−d²/(2 · 0.2²))), bounded instead of quadratic.
- Answers follow the observation model of plan section 5.2 with the probit noise σ = 1, a tie threshold of 0.2, and the latent difference d = level · Δu / sd(u), with sd(u) over the setting's feasible sweep. Levels 1, 2 and 4; level 2 is the middle level at which the criteria are judged.
- Sessions of 30 comparisons. At each session start the learner is rebuilt from every stored answer, as the page does on load; "settled" needs two sessions that settle in the same region (plan section 5.6).
- The harness runs each hyperparameter search to its end before the next pair. The page runs it in the worker's idle time and proposes the next pair first (plan section 9), so on the page the pairs after a load, an undo, a rebuild at the look cap and right after every tenth answer use the previous hyperparameters until the search ends; after a load or an undo these are the family's stored ones, and the starting values only when none are stored. A search requested while another runs waits for it and then starts on the data present then, and a rebuild keeps a running search (plan section 9); otherwise each search starts as the harness's does and ends in the same state. In the indicative bench of section 6 a whole search took about 7.6 s of idle time at 90 stored comparisons and about 55 s at the look cap. The experiment does not model this delay. The page shows its relevance ranking once its length scales come from a finished search, stored or since the load, and reports "settled" only after the first search since the load or the undo has ended.
- Budget: 180 comparisons per family for the learner (90 in the two-family acceptance run); random pairs, two distinct looks drawn uniformly from the feasible sweep and fitted by the same learner, stop at 90.
- Regret r = (u* − u(best)) / (u* − median u over the feasible sweep), where u* is the scene's optimum and "best" is the shown look with the highest posterior mean.
- Settings: low (only the four relevant parameters vary; the others stay at the optimum) and full (all 18 parameters vary).
- Seeds: development seeds 100 to 149 for the calibration of section 2; acceptance seeds 0 to 49, run once after the calibration and the code review; seeds 200 to 249, unused before, for the two-family confirmation (section 3.3).
- Commands (`--dump` resumes an interrupted run from its saved runs; the output records a digest of the code):
  - `node tools/tastelab/sim/experiment.mjs --setting full --owner quad --families 1 --noise 1,2,4 --budget 180 --tau 0.05 --stable 10 --seeds 50 --first 0 --jobs 30`
  - the same with `--setting low`; with `--families 2 --noise 2 --budget 90`; and with `--owner cross` and `--owner bump` at `--noise 2`;
  - the two-family confirmation: `--families 2 --noise 2 --budget 180 --first 200`, judged at 150 comparisons per family from the output's curve (the output's own pass flag reports the 90-comparison checkpoint).
- Code digest of the acceptance runs and the two-family confirmation: `7eb0b7fca8018935` (commit `e257f72`; the harness hashes itself and every module of `core/` and `page/`). The learner is that of commit `83f4068`; the later commit changes one check of the harness.
- Harness fix during the acceptance: the first attempt (digest `f11a4445f59d1d90`) stopped at seed 39 on the harness's check that no look of the sweep beats the optimum. The optimum's relevant values pass through the unit axis and back; for seed 39 this moved chroma by one unit in the last place, so the drawn target, which is in the sweep, beat u* by 1.8 × 10⁻³². The check gained a tolerance of 10⁻¹². Among seeds 0 to 199 the old check stopped seeds 39 and 96, and with two families also 124 and 141; none of them is in a development block that was run. Every other seed gives the same result (development seeds 100 to 102 were byte-identical). The acceptance then ran again from the start; no result of the first attempt was read.

## 2. Settled-rule calibration

The rule that chooses τ and K was fixed before any result was read (plan section 9, "Settled calibration result"). The learner's pairs and best looks depend on neither value, so one run on the development seeds 100 to 149 (full setting, quad owner, level 2, 180 comparisons in sessions of 30) replays the settled rule for the whole grid; the harness checks that the replay of the learner's own values equals its flags in every round.

| τ | K | Settled by 90 | Settled by 180 | Falsely settled by 90 | Falsely settled by 180 |
|---|---|---|---|---|---|
| 0.02 | 10 | 0 of 50 | 0 of 50 | 0 | 0 |
| 0.02 | 15 | 0 of 50 | 0 of 50 | 0 | 0 |
| 0.02 | 20 | 0 of 50 | 0 of 50 | 0 | 0 |
| **0.05** | **10** | **1 of 50** | **13 of 50** | **0** | **0** |
| 0.05 | 15 | 0 of 50 | 10 of 50 | 0 | 0 |
| 0.05 | 20 | 0 of 50 | 6 of 50 | 0 | 0 |
| 0.1 | 10 | 19 of 50 | 49 of 50 | 5 | 7 |
| 0.1 | 15 | 10 of 50 | 47 of 50 | 4 | 5 |
| 0.1 | 20 | 6 of 50 | 45 of 50 | 3 | 3 |
| 0.15 | 10 | 37 of 50 | 50 of 50 | 11 | 13 |
| 0.15 | 15 | 21 of 50 | 50 of 50 | 5 | 6 |
| 0.15 | 20 | 10 of 50 | 50 of 50 | 4 | 5 |
| 0.2 | 10 | 40 of 50 | 50 of 50 | 14 | 15 |
| 0.2 | 15 | 24 of 50 | 50 of 50 | 6 | 8 |
| 0.2 | 20 | 11 of 50 | 50 of 50 | 5 | 7 |
| 0.3 | 10 | 40 of 50 | 50 of 50 | 15 | 16 |
| 0.3 | 15 | 28 of 50 | 50 of 50 | 9 | 10 |
| 0.3 | 20 | 14 of 50 | 50 of 50 | 6 | 8 |

- Constraint: falsely settled (declared while r > 0.20) by 90 comparisons in at most 5% of seeds, that is at most 2 of 50, and by 180 comparisons in at most 10%. Only τ = 0.02 and τ = 0.05 meet it; every larger τ falsely settles 3 or more seeds by 90 comparisons.
- Objective: the most seeds settled by 90 comparisons, then by 180, then the smaller τ, then the larger K. Chosen: **τ = 0.05 and K = 10**, settled by 90 comparisons in 1 of 50 seeds and by 180 in 13, never falsely.
- The run used digest `75fef8fb56b23b4c` (commit `d3662cf`). The acceptance code differs from it in `page/app.js` and `page/worker.js`, which the harness does not import, and in the harness's optimum check (section 1). Its results equal, seed by seed, those of the development run that chose the learner (plan section 9).
- The first calibration, with an unchanged best look as condition (2), never settled at any grid point; condition (2) became stability by region before this run (plan section 5.6).
- On the fresh development seeds 150 to 199 the same rule would have chosen τ = 0.3 and K = 20 (settled by 90 comparisons in 16 of 50 seeds, falsely in 2). The rule uses seeds 100 to 149 only.
- With these values "settled" seldom fires by 90 comparisons. Criterion 4 limits wrong settled states, not their absence.

## 3. Result

All runs: code digest `7eb0b7fca8018935`, 50 paired seeds, sessions of 30, τ = 0.05 and K = 10. Every dump line carries this digest, and no run wrote error output. Bold: the middle noise level, at which the criteria are judged.

### 3.1 Full setting, one family

| Noise level | Median r at 90, learner | Median r at 90, random pairs | Wilcoxon p | r ≤ 0.10 at 90 | Strict ranking at 160 | Falsely settled by 90 | Settled by 90 / 180 | Median r at 180 |
|---|---|---|---|---|---|---|---|---|
| 1 | 0.227 | 0.301 | 0.0034 | 28% | 26% | 2% | 2% / 10% | 0.073 |
| **2** | **0.070** | 0.180 | **0.0005** | 64% | **82%** | **0%** | 0% / 20% | 0.030 |
| 4 | 0.020 | 0.109 | 1.2 × 10⁻⁷ | 98% | 96% | 0% | 0% / 70% | 0.009 |

At the middle level all four criteria pass. The median r is 0.270, 0.132, 0.070, 0.048 and 0.030 after 30, 60, 90, 120 and 180 comparisons; the strict ranking holds in 48%, 64%, 82% and 84% of seeds after 90, 120, 160 and 180.

### 3.2 Low setting

| Noise level | Median r at 90, learner | Median r at 90, random pairs | Wilcoxon p | r ≤ 0.10 at 90 | Strict ranking at 160 | Falsely settled by 90 | Settled by 90 / 180 | Median r at 180 |
|---|---|---|---|---|---|---|---|---|
| 1 | 0.264 | 0.179 | 0.99 | 20% | 22% | 22% | 40% / 74% | 0.135 |
| **2** | **0.093** | 0.094 | **0.76** | 54% | **62%** | **18%** | 44% / 98% | 0.029 |
| 4 | 0.018 | 0.096 | 6.3 × 10⁻⁷ | 90% | 82% | 14% | 66% / 100% | 0.008 |

At the middle level the median regret passes; the comparison with random pairs, false "settled" states and the ranking miss.

- Ranking: the 14 parameters held at the optimum never vary, so the answers carry no information about them and their length scales stay near the prior. The check then asks whether every relevant length is shorter than the prior's, not whether relevant and irrelevant parameters are told apart. The 2 October report already noted that it cannot pass in this setting.
- Random pairs: with four varying parameters, random pairs are informative on their own (median r 0.094 at 90, against 0.180 in the full setting). The learner beats them at level 4 (0.018 against 0.096) and does worse at level 1 (0.264 against 0.179). The learner of 2 October did not beat them in this setting either.
- Settled: the rule was calibrated in the full setting only (section 2). In the low setting it fires early, by 90 comparisons in 44% of seeds against none in the full setting, and wrongly in 18%.
- The page has no mode in which only some parameters vary. The low setting and the two families had not been run on development seeds with the final learner before the acceptance.

### 3.3 Two families

| Comparisons per family | Seeds | Median of max(r_A, r_B), learner | Same, random pairs | Both families at r ≤ 0.10, learner / random | Wilcoxon p |
|---|---|---|---|---|---|
| 90 (acceptance) | 0 to 49 | 0.123 | 0.290 | 46% / 8% | 6.3 × 10⁻⁷ |
| **150 (confirmation)** | 200 to 249 | **0.045** | – | 90% / – | – |

- At 90 comparisons per family the criterion misses. Each family alone reaches r ≤ 0.10 in 68% and 72% of seeds (median r 0.067 and 0.049), as one family does alone (64%, section 3.1); both together in 46%, close to the 49% expected if the two were independent. The families do not interfere; the criterion asks both to succeed, which for two independent families needs about 71% of seeds per family.
- The owner kept the threshold and moved the checkpoint to 150 comparisons per family. The confirmation ran once on the unused seeds 200 to 249 with the same code, 180 comparisons per family, judged at 150; its rule was fixed before the run. Random pairs stop at 90 comparisons, so they have no value at 150. It passes: both families are within 0.10 in 45 of 50 seeds. On these seeds the median of the larger regret is 0.106 at 90 comparisons per family (both families within 0.10 in 44% of seeds; random pairs 0.239), 0.084 at 120 (62%), 0.045 at 150 (90%) and 0.042 at 180 (92%); the miss at 90 repeats here. By 150 comparisons per family the settled rule fired falsely in 0 and 2 of 50 seeds (by 90 in 0 and 1). Every dump line carries the digest above, and the run wrote no error output.

## 4. Comparisons before the owner starts

- One family (full setting, middle level, section 3.1): after 90 comparisons the median regret is 0.070 and 64% of seeds are within 0.10; after 120, 0.048 and 66%; after 150, 88% of seeds; after 180, 0.030 and 96%. The ranking of what matters is provisional until 160 comparisons, and the page says so.
- Two families: about 150 comparisons per family, 300 in total. On the confirmation seeds the median of the larger regret is 0.045 at 150 comparisons per family, with both families within 0.10 in 90% of seeds; at 120 per family 0.084 and 62%; at 90 per family 0.106 and 44% (acceptance seeds: 0.123 and 46%).
- Each seed of the experiment runs in one scene. The page has three scenes; how many comparisons the shared parts of the utility save across them is not measured, so these counts are per family and scene.

## 5. Robustness owners

Middle noise level, one family, full setting.

| Owner | Median r at 90, learner | Median r at 90, random pairs | Wilcoxon p | r ≤ 0.10 at 90 | Strict ranking at 160 | Falsely settled by 90 | Settled by 90 / 180 | Median r at 180 |
|---|---|---|---|---|---|---|---|---|
| cross | 0.075 | 0.216 | 0.0002 | 64% | **58%** | 0% | 0% / 4% | 0.040 |
| bump | 0.077 | 0.253 | 0.0003 | 60% | 80% | 2% | 2% / 70% | 0.023 |

- Bump passes all four criteria; the ranking at 160 is at the threshold (40 of 50 seeds; 45 of 50 on development seeds 100 to 149).
- Cross passes three criteria. Its median regret at 90 is lower than on development seeds (0.123 and 0.107, section 8), but the strict ranking at 160 holds in 58% of seeds (development: 74% and 70%) and in 74% at 180. With interacting preferences the ranking is not reliable at 160 comparisons. The owner recorded this as a known limitation: the ranking panel is a hint, and the presets and the export do not depend on it.

## 6. Latency (indicative)

Headless Chrome 154 with a throwaway profile and a local server on 127.0.0.1, on the development machine (32 logical processors). The bench runs the learner in a module worker with the worker's own flow: an answer, then the next pair; the hyperparameter search one evaluation per idle step. Records are synthetic feasible looks with random answers.

| Stored comparisons | Looks | Answer to next pair, median (max) over 12 answers | One idle search step, median (max) | Whole search |
|---|---|---|---|---|
| 90 | 180, then 194 | 263 ms (354 ms) | 13 ms (18 ms) | about 7.6 s in 502 steps |
| 200 | 400, the cap (2 rebuilds) | 585 ms (757 ms) | 102 ms (122 ms) | about 55 s in 537 steps |

In this run the next pair was ready within the 1 s target of plan section 5.5 at the maximum size. An answer that arrives during an idle step waits for that step, here at most about 0.12 s. The figures are indicative only; they make no performance claim.

## 7. Open items

- Low setting (section 3.2), a known limitation by owner decision: before any page mode narrows the parameter space, such as a parameter-space editor, the settled rule is calibrated again in the narrowed space, and the ranking check is defined for parameters that never vary.
- Ranking with interacting preferences (section 5), a known limitation by owner decision: the ranking panel is a hint; at 160 comparisons the cross owner's ranking held in 58% of seeds.

## 8. History

- 2 October 2026: the learner as handed over missed every criterion at 60 comparisons in both settings (full setting, middle level: median r 0.402, no better than random pairs). Two harness defects were fixed as bugs by owner decision: the noise level was about half the nominal one, and the random baseline drew from the model's own candidates.
- An escalated diagnosis replaced the kernel, the priors, the hyperparameter search and the pair choice (plan sections 5.1 and 5.3 to 5.5). The relevance ranking still missed at 60 comparisons (40% and 20% on two development blocks); on 3 October 2026 the owner kept the thresholds and moved the checks to 90 comparisons (ranking at 160, provisional before), and had the settled rule recalibrated: the first rule never fired.
- 3 October 2026: the cross and bump owners showed that the additive quadratic kernel falls short when terms are bounded (bump: median r 0.133 at 90 on development seeds); the kernel gained a smooth part and an interaction part, each weighted by the evidence.
- 3 October 2026, later: the strict relevance ranking at 160 comparisons held in 32 of 50 development seeds. Hue rotation, the only relevant circular parameter, had no second-order term, and lengths from a grid of factor 1.5 often tied. A quadratic term for circular parameters and refinement steps between the grid values raised it to 40 of 50 (fresh development seeds 150 to 199: 40 of 50 against 36 of 50), at a cost in regret at 90 comparisons that is not significant (median 0.068 against 0.058 over 100 seeds). The settled rule was then calibrated (section 2).
- 3 October 2026, after the calibration: on development seeds the cross owner missed two criteria (median r 0.123 and 0.107 at 90 comparisons; strict ranking at 160 in 74% and 70% of seeds). A plan check set the order of the diagnosis and a limit of two variants. Refits of the same answers found no change that could ship: interactions weighted by length gave 0.125 at 90 and interactions only between the two shortest lengths 0.139, against 0.130 for a fresh fit of the current model; without interactions r was 0.125 and the ranking at 160 fell to 23 of 50 from 38. Diagnostic fits that use knowledge of the owner reached 0.112 to 0.123. By 90 comparisons a look with r of at most 0.10 had been shown in 47 of 50 seeds, but the posterior placed it below the reported best (by a median of 1.4 posterior standard deviations), mostly along the shallow chroma–gap valley. Candidate moves that change two short-length parameters together left cross unchanged (0.119 at 90, lower in 25 seeds and higher in 25; ranking at 160 in 37 of 50) and quad unchanged (0.053 against 0.064, two-sided p 0.80). The miss went to the owner before the acceptance run.
- 3 October 2026, acceptance on seeds 0 to 49 (this report): the full setting with one family and the bump owner passed every criterion; the cross owner missed the ranking at 160 comparisons, two families missed at 90 comparisons per family, and the low setting missed three criteria. The owner accepted the learner with these as recorded limitations and moved the two-family checkpoint to 150 comparisons per family, where one confirmation on fresh seeds passed (section 3.3).
- 3 October 2026, after the acceptance: the page lost its fitted hyperparameters on every load, undo and rebuild at the look cap, so pairs came from the starting values until the next search ended, and near the cap the settled rule could run with length 1 on every parameter. The page now stores each family's hyperparameters after every finished search, uses them until the next search ends, and reports "settled" only after that search (plan section 9). Every search still starts where it did, never from stored values, so it ends as before: the harness gives the same results with the new code (digest `22f17580cfd4ea6c`) as with `7eb0b7fca8018935` in three runs of the full setting at the middle noise level on seeds 100 and 101: the quad owner with two families and 60 comparisons, the cross owner with one family and 60, and the quad owner with one family and 180, where "settled" fires.
- 3 October 2026, later: the export finished every family's pending search first, near the look cap about a minute per family in the bench of section 6, and the worker answered nothing meanwhile. It now uses each family's hyperparameters in force and finishes a search only for a family whose values come from no finished search; as on the page, nothing is settled before a search since the load has ended (plan section 7). The plan check of this change found that a search replaced by the next one before it ended handed its point to the model in place of the stored values, and the pairs, the relevance ranking and the best looks then used that point as fitted. Stored values now stay in force until a search ends, and a replaced search only sets where the next one starts (plan section 9). The harness gives the same results with the new code (digest `9c102a9cd4aaa11f`) in the same three runs.
- 3 October 2026, later still: near and at the look cap a fast owner could keep every search from ending. A tenth answer replaced a running search, and at the cap almost every answer rebuilt the learner, which dropped its search and cleared `searched`; "settled" then waited, and the stored hyperparameters were not renewed. A search requested while another runs is now queued and starts when the running one ends, and a rebuild keeps a running search and `searched` (plan section 9). The harness never has a search running when it requests one and gives the same results with the new code (digest `117d9f157587a04b`) in the same three runs.
- The 2 October report (failure, harness defects, diagnosis and variants) is in this file's history.
