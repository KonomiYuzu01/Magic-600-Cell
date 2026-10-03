# Taste Lab phase 1: simulated-owner experiment

Status: **failed; owner decision pending** (2 October 2026). The learner as handed over (commit `c75f305`) does not meet the pass criteria of [plan section 6](taste-lab-plan.md#6-simulated-owner-experiment) in either setting. As the plan requires, the result goes to the owner before the learner is offered as one. The criteria are unchanged.

Evidence class: synthetic simulation only, run headless in Node.js 24.21.0 on Windows. No owner answers, sessions or images were used. Latency figures are indicative only (Node, not headless Chromium, and not the owner's laptop).

## 1. Setup

- Feasible sweep: 247 of 16,384 Sobol looks pass the hard checks (1.5%) with the default parameter table.
- Synthetic owner: utility u = −Σ w·(x − c)² over four relevant parameters on their unit axes: hue rotation (circular, w = 1), chroma (1.5), sticker gap (1) and glow (1). The optimum c is a feasible look drawn per seed; in the celebrating scene the preferred glow moves by 0.35. The other 14 parameters are irrelevant. Each seed uses one scene.
- Answers follow the observation model of plan section 5.2 with a tie threshold of 0.2.
- Budget 60 comparisons, 50 paired seeds, noise levels 1, 2 and 4.
- Low setting: only the four relevant parameters vary; the others stay at the optimum. Full setting: all 18 parameters vary.
- Regret r = (u* − u(best)) / (u* − median u over the feasible sweep), where "best" is the shown look with the highest posterior mean.
- Command: `node tools/tastelab/sim/experiment.mjs --setting low|full --budget 60 --seeds 5 --first <k>` for k = 0, 5, …, 45, merged.

## 2. Result as handed over

| Setting | Noise level | Model median r | Random median r | Wilcoxon p | Relevance ranked | Settled (wrong) |
|---|---|---|---|---|---|---|
| low | 1 | 0.395 | 0.320 | 0.86 | 0% | 0 (0) |
| low | **2** | **0.194** | 0.272 | 0.49 | 0% | 0 (0) |
| low | 4 | 0.095 | 0.082 | 0.58 | 0% | 0 (0) |
| full | 1 | 0.473 | 0.491 | 0.43 | 0% | 0 (0) |
| full | **2** | **0.402** | 0.408 | 0.61 | 0% | 0 (0) |
| full | 4 | 0.330 | 0.260 | 0.57 | 0% | 0 (0) |

At the middle noise level both settings fail the regret target (0.10), the comparison with random pairs and the relevance ranking. "Settled" is never declared, so its error rate of 0% passes only trivially.

## 3. Two defects in the experiment

1. **Noise scale.** The harness computes d = snr·Δu/0.5, but the standard deviation of u over the feasible sweep is about 0.28 (median over seeds: 0.284 full, 0.272 low). The latent signal-to-noise ratio is therefore about 0.56 times the nominal level: "2" is about 1.1. Plan section 5.1 fixes the probit noise at σ = 1, so the level should be the latent signal amplitude over σ. Correction: d = snr·Δu/sd(u), with sd(u) over the feasible sweep for that owner and scene.
2. **Random baseline.** The handed-over baseline draws both looks from the round's candidates, which include up to 256 perturbations of the model's current best: a model-guided local search, not random pairs. Correction: two distinct looks drawn uniformly from the setting's feasible sweep. Both methods still recommend the posterior-mean best of a model fitted to their own answers.

Both corrections make the test easier for the model, so they are part of the owner decision and not applied to the committed harness yet.

## 4. Result with both corrections (noise level 2)

| Setting | Model median r | Uniform random median r | Wilcoxon p | Relevance ranked | Settled (wrong) |
|---|---|---|---|---|---|
| low | **0.097** | 0.131 | 0.051 | 0% | 0 (0) |
| full | **0.296** | 0.517 | **0.00026** | 0% | 0 (0) |

The low setting meets the regret target but does not beat random pairs at p < 0.01. The full setting beats random pairs but misses the regret target by a factor of three. In the low setting the relevance check cannot pass: the fixed parameters carry no data, so their length scales stay near the prior.

More comparisons do not close the gap in the full setting. Median regret by budget (50 seeds):

| Comparisons | 10 | 30 | 60 | 100 | 150 |
|---|---|---|---|---|---|
| Model | 0.392 | 0.363 | 0.296 | 0.267 | 0.263 |
| Seeds with r ≤ 0.10 | 6 | 4 | 5 | 10 | 12 |

## 5. Diagnosis

Shown looks are good; the pick among them is not.
- After 60 comparisons the best look shown so far has median regret 0.070 (full) and 0.026 (low), against a recommendation of 0.296 and 0.097.
- The median look is compared once, and so is the best look shown. The model keeps showing new looks and rarely revisits the best ones.

The model does not learn which parameters matter from 60 answers (20 seeds full, 5 seeds low, corrected noise).
- After 60 answers every length scale sits near 0.225, one search step below the starting value 0.5, close to the prior mode (0.18).
- A search with 1,000 instead of 60 evaluations gains less than one nat and ends at 0.18 to 0.22 for every parameter.
- The fitted evidence never prefers the true relevance (relevant lengths 0.35, irrelevant 5) over the as-run fit: 0 of 35 seeds.
- Given the true relevance and the true signal amplitude, the pick from the same answers is still 0.171 in the full setting (5 of 20 seeds reach 0.10). The answers themselves, about one comparison per look, limit the pick.

## 6. A model variant: length-scale prior scaled with the dimension

The hyperprior puts the length scales at a median of 0.5 with its mode at 0.18. With 20 features at such lengths, two random looks are almost uncorrelated, so the model treats every look on its own. The variant uses a log-normal prior whose median grows with the square root of the dimension ([Hvarfner, Hellsten and Nardi, ICML 2024](https://arxiv.org/abs/2402.02229)): median exp(√2 + ½·ln 20) ≈ 18 (the bound stays at 5), log standard deviation √3, mode about 0.9. The search starts at length 1. Both experiment corrections apply. Noise level 2, 60 comparisons, 50 paired seeds:

| Setting | Model median r | Uniform random median r | Wilcoxon p | Relevance ranked | Best look shown |
|---|---|---|---|---|---|
| low | 0.159 | 0.146 | 0.51 | 2% | 0.042 |
| full | **0.158** | 0.295 | **0.00001** | 0% | 0.041 |

The full setting improves from 0.296 to 0.158 and beats random pairs in 43 of 50 seeds, but still misses 0.10; the low setting gets worse.

Two further checks, full setting, noise level 2:
- Playoff rounds (every fifth round compares the two of the top four shown looks whose difference is least certain): 0.178 with the scaled prior and 0.232 with the original prior, against 0.158 and 0.296 without them. In the low setting they make both priors worse (0.199 and 0.124). Not adopted.
- Search budget (20 seeds, scaled prior): with 300 instead of 60 evaluations per refit the median regret is 0.133, and the relevant parameters rank first in 4 of 20 seeds (0 with 60 evaluations). Relevant length scales then have a median of 0.82, irrelevant ones 1.49: the signal is there but too weak at 60 answers to rank all four first.

No variant meets all four criteria. The relevance ranking stays far below 80% even with five times the search, so it is limited by the number of answers, not by the search.

## 7. Latency (indicative)

Plan section 5.5 sets 1 s for the next pair at the maximum size.

| Looks | Comparisons | Next pair | Hyperparameter refit (every 10 answers) |
|---|---|---|---|
| 60 | 60 | 0.30 s | 0.05 s |
| 150 | 150 | 0.85 s | 0.36 s |
| 400 | 600 | 4.0 s | 5.8 s |

Building the feasible pool at page start takes 0.65 s. At 400 looks the time goes to `predict`, which always computes a full covariance: choosing the best look needs only the mean but takes 0.65 s, and the candidates' joint posterior is computed twice per round (next pair 1.5 s, settled test 1.05 s).

## 8. Decision needed

Put to the owner on 2 October 2026; the decision will be recorded in the wiki.
- Whether the two experiment defects (section 3) are fixed as bugs.
- The next step:
  - offer the page as a browsing aid: the scaled prior, no "what matters" panel and no "settled" claim, the failed experiment recorded;
  - grouped sessions (a few parameters per session), rerun per group with the criteria unchanged;
  - new criteria set by the owner, then a rerun;
  - pause phase 1a.
- In every case the latency fix follows (section 7): the best look from the posterior mean only, and one joint posterior per round.
