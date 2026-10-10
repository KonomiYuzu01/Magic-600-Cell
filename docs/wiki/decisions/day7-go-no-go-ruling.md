---
id: day7-go-no-go-ruling
type: decision
status: verified
visibility: public
summary: Astra's day-7 go/no-go gate ruling of 10 October 2026. S-B, S-A2 (Godot) and S-D (Qt) all go on their W3 results with the area-centroid anchor. The ruling covers those three build identities only, selects no renderer, and records three minor risks (Qt's capped headroom, the open H-06 and H-09, and the guard's residual limits).
related: [renderer-candidates, s-b-probe-results, level-2-framework-results, owner-decisions-2026-10-10-anchor]
supersedes: []
claims:
  - {id: ruling, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: gate-packet, evidence_kind: decision, path: docs/progress/1.0/packets/renderer/E-2.4-04-day7-gate.md, sha256: b10b505960c96dc3ad42ba57abdefd51b0dd3018788ce7549da3f02bcfad038c, checked_at: 2026-10-10}
  - {id: sb-w3-met, evidence_kind: performance, path: work/experiments/renderer-sb/results/w3-20261010T164807115Z-summary.json, sha256: 5d3752b3543b443ad29339ae16771bd56d8044876f503c65ae7d6650a39c2cac, checked_at: 2026-10-10}
  - {id: sa2-w3-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json, sha256: 339f629cc1197f5cd35a17337a3166f69f6c5769523e073f8af718bec07d43ed, checked_at: 2026-10-10}
  - {id: sd-w3-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sd-w3-20261010T172910231Z-summary.json, sha256: e990333279f6fe70ecc122059d247036945933b5f3f8ce7b98c6c921909ce52e, checked_at: 2026-10-10}
---

# Day-7 go/no-go ruling, 10 October 2026

Codex Astra gave the final ruling at the day-7 gate on a filled copy of packet [E-2.4-04](../../progress/1.0/packets/renderer/E-2.4-04-day7-gate.md):
- call `20261010T213742Z-79856ae3`;
- model `gpt-6-astra`, effort `ultra`, standard tier.

The owner may override it only by an explicit exception that names the fix and its deadline. The gate is the one in [renderer-candidates](renderer-candidates.md).

## Ruling

| Candidate | Ruling | Build identity | Deciding summary | Pooled W3 |
|---|---|---|---|---|
| S-B, bare Direct3D 12 | **go** | `d41020f6…` | [w3-20261010T164807115Z](../../../work/experiments/renderer-sb/results/w3-20261010T164807115Z-summary.json) | 788.16 fps, p99 1.493 ms |
| S-A2, Godot 4.7.2 .NET | **go** | `06d14019…` | [sa2-w3-20261010T175043422Z](../../../work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json) | 670.69 fps, p99 1.907 ms |
| S-D, Qt 6.10.3 | **go** | `befc441f…` | [sd-w3-20261010T172910231Z](../../../work/experiments/renderer-l2/results/sd-w3-20261010T172910231Z-summary.json) | 60.00 fps, p99 17.440 ms |

- **Which builds decide.** The 10 October builds with the area-centroid shrink anchor decide. The [anchor decision](owner-decisions-2026-10-10-anchor.md) requires W3 re-acceptance for changed builds. It keeps the earlier results only for their own identities.
- **What the ruling checked:**
  - the summary digests match;
  - each series has three distinct runs of one build identity;
  - every run and the pooled frames pass, over the 180 s interval;
  - no run is invalid or unreadable;
  - the recorded conditions meet the gate: native resolution, mains power, no frame generation and no upscaling.
- **Cold runs, attendance and label checks** are documented on the result cards: every run passed its label check with 1,010 revisions. The judge's own rules reject a run whose label check failed.
- **Godot's single 12.33 ms frame**, in run 1, does not violate the gate.
- **Discrete GPU.** The level 2 records show that the NVIDIA GPU presents. They do not document a manual NVIDIA Control Panel check; only the S-B summaries carry the operator's MUX declaration.
- The ruling did not reproduce the private captures independently.
- **Scope.** The ruling holds for the three named builds and the recorded conditions on the owner's machine. It selects no renderer: that is [E-2.4-05](../../progress/1.0/packets/renderer/E-2.4-05-selection.md) on stage day 14 (H-10). It does not show readiness for W5.

## Recorded risks

Each finding was adopted. None blocks continuation.

| ID | Severity | Risk | Experiment that settles it |
|---|---|---|---|
| D7-001 | minor | Qt's 60 fps is its vblank cap. It shows no headroom and cannot rank Qt against S-B or Godot. | A new Qt build identity with `QT_D3D_NO_VBLANK_THREAD=1` and `QT_QPA_UPDATE_IDLE_TIME=0`, then three attended cold W3 runs |
| D7-002 | minor | The H-06 feature costs, the H-09 framework constraints and S-B's W1, W2 and W4 attribution runs are open. Bare W3 can pass while the design's look fails W5. | Measure H-06, report H-09, and run three formal W5 runs when the design handoffs arrive |
| D7-003 | minor | The guard's attended experiments have not run: a replacement helper during a W3 run, and the Qt `module-*` injections. Its observer does not record loads through out-of-scope aliases. | Run [PLAN-L2-V-002](../../../work/experiments/renderer-l2-packets/PLAN-L2-V-002.md) section 7, items 2 and 3 |
| D7-004 | nit | A sentence on the level 2 card said no gate run had used the guard. | Fixed: the sentence is now dated as written on the morning of 10 October |

## What follows

- **H-07 to the design track: all three candidates continue.** Godot, the Look Lab framework, stays a candidate, so the Look Lab remains a valid place for the design work, including G6. The selection on stage day 14 can still choose another candidate. In that case the framework-neutral rebuild of the design outputs applies.
- **Window days 8 to 11 (stage days 10 to 13).** Each candidate runs W5 with the design handoffs (H-01 to H-04): three formal runs each of W3 and W5 ([renderer-experiment-plan](../../progress/1.0/renderer-experiment-plan.md) section 3).
- **Next attended GPU session:**
  - H-06, then S-B's W1, W2 and W4;
  - the experiments of D7-001 and D7-003.

  The level 2 runner should also record the discrete-GPU declaration, as the S-B runner does.
