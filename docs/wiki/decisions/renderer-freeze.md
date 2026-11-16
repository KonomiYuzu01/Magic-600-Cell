---
id: renderer-freeze
type: decision
status: draft
visibility: public
summary: Draft renderer freeze record for the owner's signature - S-A2, the S-B Direct3D 12 drawing core as a shared native library hosted by Godot 4.7.2 .NET, selected on build 06d14019 and its three cold W3 runs; W-J on the selected candidate, the product build's gate run and the RPI.80 statement as conditions; risks and deferred items; later changes follow PHS.52. Nothing is frozen until the owner signs.
related: [owner-decisions-2026-11-16-renderer-close, renderer-candidates, day7-go-no-go-ruling, owner-decisions-2026-10-22, owner-decisions-2026-10-10-anchor, s-b-probe-results, level-2-framework-results, w-j-probe-results]
supersedes: []
claims:
  - {id: freeze-draft, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: sa2-w3-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json, sha256: 339f629cc1197f5cd35a17337a3166f69f6c5769523e073f8af718bec07d43ed, checked_at: 2026-11-16}
  - {id: sb-w3-met, evidence_kind: performance, path: work/experiments/renderer-sb/results/w3-20261010T164807115Z-summary.json, sha256: 5d3752b3543b443ad29339ae16771bd56d8044876f503c65ae7d6650a39c2cac, checked_at: 2026-11-16}
  - {id: wj-w3-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/w3-20261010T194430557Z-summary.json, sha256: 3cc627af916c9e564851d57b2c853ef389e24ddf5b634629908ce6e0cfa7c868, checked_at: 2026-11-16}
  - {id: wj-s4-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/wj-S4-20261010T184006692Z-summary.json, sha256: 54b4bf3b1ad1cabc6b6bd589288ddfbe6f2e2c7972c7b7d38cdfdf7eb105bff1, checked_at: 2026-11-16}
  - {id: wj-ia-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/wj-I_a-20261010T191829654Z-summary.json, sha256: e622ec1a3b328074d1a584f7efbb359a786369d7f6207ca83ec61a4e53c1d3f7, checked_at: 2026-11-16}
  - {id: wj-ib-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/wj-I_b-20261010T193029102Z-summary.json, sha256: 3ce21f3f2b5d70644bc5c013f5517e9c20fd904b3cce5f85ba6d4b34c6857330, checked_at: 2026-11-16}
---

# Renderer freeze record (draft)

Status: **draft for the owner's signature, 16 November 2026.** Nothing on this page is frozen until the owner signs.
The signature accepts [ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md) and records this freeze
(`AGENTS.md`, "Ask the owner": accepting ADRs). Owner decision:
[close the renderer line](owner-decisions-2026-11-16-renderer-close.md). Selection record:
[renderer-selection](../../progress/1.0/renderer-selection.md).

## What is frozen

| Item | Value |
|---|---|
| Renderer | S-A2: the S-B Direct3D 12 drawing core as the shared native library, hosted by Godot 4.7.2 .NET (ADR-R01.1, ADR-R01.2) |
| Drawing method | S-B's: its readers, checks and unmodified HLSL shaders, compiled offline by DXC to DXIL, with the area-centroid shrink anchor ([owner, 10 October 2026](owner-decisions-2026-10-10-anchor.md)) |
| Host as tested | the official Godot 4.7.2 .NET editor binary running the project, on .NET 8 (`net8.0`) |
| Selection build | `06d14019c937ed55e90ad3a78e28e9988139a3fdd946e3c9f20495e9ee034106`, covering the native library, its shaders, the recorded framework files and the settings; three cold W3 runs on 10 October 2026 (G1) |
| Same core, other hosts | the S-B probe `d41020f6…` (B1), and its copy with the per-piece pose path `74b22fec…`, on which W-J items 5 and 6 are met (B2 to B5) |
| Gate | unchanged ([renderer-candidates](renderer-candidates.md)) |
| Workload | W3 in place of W5 (E-2.4-05): the opaque baseline, with no H-06 feature and no look |

The freeze covers the choice of renderer and its structure, a core and a host. It freezes no product build: the
product build gets its own identity and passes the unchanged gate before any support is advertised (RPI.79).

## Conditions

1. **W-J on the selected candidate** (ADR-R01.3): option A or option B, as the owner chooses at the signature. Under
   option A this is a conditional freeze in PHS.52's sense: a failing run reopens the selection, and the owner decides.
2. **The product build** passes the unchanged gate in one run with the advertised combination before any stage or view
   support is advertised (RPI.79, RM.12). It has a release export template, a supported .NET release, the pose path
   and the core's features.
3. **RPI.80.** The statement in ADR-R01.4 stands for this renderer. Shipping a run-time compiler stays the owner's
   decision, and until then no user stage is advertised.

## Evidence

| Claim | Summary | Result |
|---|---|---|
| `sa2-w3-met` | [G1](../../../work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json) | S-A2, `06d14019…`: 671.28, 671.05 and 669.74 fps, pooled 670.69; p99 1.895, 1.906 and 1.916 ms, pooled 1.907 ms; peak VRAM 217.2 MB |
| `sb-w3-met` | [B1](../../../work/experiments/renderer-sb/results/w3-20261010T164807115Z-summary.json) | S-B, `d41020f6…`: pooled 788.16 fps, p99 1.493 ms; peak VRAM 81.7 MB |
| `wj-w3-met` | [B2](../../../work/experiments/renderer-wj/results/w3-20261010T194430557Z-summary.json) | S-B with the pose path, `74b22fec…`: pooled 610.28 fps, p99 1.861 ms |
| `wj-s4-met`, `wj-ia-met`, `wj-ib-met` | [B3](../../../work/experiments/renderer-wj/results/wj-S4-20261010T184006692Z-summary.json), [B4](../../../work/experiments/renderer-wj/results/wj-I_a-20261010T191829654Z-summary.json), [B5](../../../work/experiments/renderer-wj/results/wj-I_b-20261010T193029102Z-summary.json) | `74b22fec…`: S4 614.96 fps, p99 1.926 ms; I_a 613.28 fps, p99 1.911 ms; I_b 611.92 fps, p99 1.920 ms |

Every run passed its exact label check, or its exact pose check in the W-J runs. These are actual Windows/DirectX and
performance results for the identities named. They say nothing about a look, a feature or a product build.

## Risks

| Risk | Mark |
|---|---|
| W-J item 5 and the W3 re-acceptance are unrun on the selected candidate's pose-path build | condition 1 |
| No confirmed-present route yet (RM.26 and ONE.41, required by RPI.66). Godot 4.7.2's public API has no swap chain handle, and an engine change needs a Godot source build, which is an owner item | record; LP3 tests it (CT-RM-06, CT-ONE-26), and a required-feature conflict returns to the owner (RPI.17) |
| The evidence ran on Godot's editor binary; release export templates are untested | condition 2 |
| .NET 8 left Microsoft's support on 10 November 2026; the move to a supported release is untested | condition 2 |
| No look is measured: charter E1 stays open, and H-06's costs are unknown | record |
| No run-time compiler yet; shipping DXC is an owner item | condition 3 |
| Device-loss recovery is untested, resize with a scene loaded is unsupported, and Godot is system DPI aware only | record; work for LP3 and the vertical slice |
| The selection build's binding rests on the loaded-file guard, whose attended experiments are unrun (D7-003) | record |
| One 12.328 ms frame in G1's run 1, against the 33.3 ms limit | record |
| G1 lacks the operator's discrete-GPU declaration; the presenting adapter is recorded | record |

## Deferred items

| Item | Reason | Taken up by |
|---|---|---|
| D7-001: uncapped Qt | S-D is not selected | the selection, only if it reopens toward S-D |
| D7-002: S-B's W1, W2 and W4 | attribution scenes, not gate scenes | none planned |
| D7-002: H-06 cost table | no feature list exists while the design line is on hold; the cost scene and the table tool are committed | the design line, with the feature list |
| D7-003: the guard's attended experiments | they test the level 2 harness, not a candidate | none planned; the product build gets its own identity |
| W-J item 7: overlay costs | the overlays' look and costs are design-line work | the design line |
| W5 | no preset arrives while the design line is on hold | the vertical slice (E1) and RPI.79's run |

## Handed on

- **To the design track:** [H-10](../../progress/1.0/renderer-handoff-H-10.md). The vertical slice and the living test
  of stage 2.4 stay open with the design line ([protocol](../../progress/1.0/stage-2-experiment-protocol.md), section
  4.3).
- **To LP3, at C9** ([LP3 plan](../../progress/1.0/language-lp3-plan.md)): the W-J runs under option A, RPI.79's gate
  run, and the windows tests, among them CT-RM-05 (device loss), CT-RM-06 and CT-ONE-26 (the presented-frame report)
  and CT-RPI-31 (the stopped-frame skip).
- **To the owner, not decided by this freeze:** shipping DXC; a Godot source build, if the confirmed present needs
  one; the design-system ADR; the feature list.

## Later changes

A later change to the frozen renderer, its host, the core's interface or its features follows chapter 12's PHS.52: "A
changed format, layer boundary or renderer feature after a conditional freeze follows its recorded revision/review and
owner acceptance process." Calling it a later phase does not bypass that process. The frozen clauses that say no
renderer is selected (RPI.60, RM.29 and chapter 10's open table) are read together with this record until a versioned
change updates their wording.

## Signature

Pending. When the owner signs, this section records the owner's own words, the date and the W-J option, and this
page's status becomes `verified`.
