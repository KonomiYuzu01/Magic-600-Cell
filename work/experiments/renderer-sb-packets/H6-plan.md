# Plan check packet: the H-06 feature cost table (E-2.4-01 item 5)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check the plan for the H-06 cost table before two Codex implementation calls start. The table gives the frame time each candidate visual feature adds when it is switched on alone, at full detail in W3 on the owner's RTX 4070 Laptop GPU. The features are sticker gaps, outlines, per-sticker transparency with sorting, fog, depth of field, ambient occlusion and MSAA 4x.
- The plan has two packets, run in parallel with disjoint files:
  - `work/experiments/renderer-sb-packets/H6-P-probe-features.md`: the features in the S-B probe;
  - `work/experiments/renderer-sb-packets/H6-T-cost-table.md`: the gate's feature grouping and `tools/perf/feature_costs.py`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample: what the plan would produce, and why that is wrong for H-06 or for the gate. Verdict `pass` if there is none.
- Check these questions:
  1. Would the measured differences be the features' costs?
     - Each feature is measured against a baseline of the same build, which is measured again.
     - Gaps are measured as T(none) minus T(no-gaps), because the gated look already has gaps.
     - Transparency covers all 259,800 stickers.
     - Depth of field and ambient occlusion are full-resolution post-processes.
  2. Does any feature definition make the measurement unrepresentative or invalid? Examples: the transparency draw order, the outline edge rule, or the effect check.
  3. Is the gate change safe?
     - Runs are grouped by (candidate, scene, feature), and a missing feature means `none`.
     - Verdict rules stay unchanged.
     - `upscaling` and `msaa` are published.
  4. Is the preliminary self-timed mode honest? It uses the probe's trace instead of PresentMon, with every other gate rule. Its results are always labelled preliminary.
  5. Is anything missing that the E-2.4-05 selection packet needs from H-06? For example, the rule that a feature that alone breaks the gate is returned to the design track with its cost.
- Out of scope:
  - code quality;
  - the shader algorithms beyond what affects the measurement;
  - `minor` and `nit` findings;
  - the choices in section 6.

## 2. Actual problem and reproduction
- The S-B probe met the gate in W3 on 3 October 2026 (build `2b5bf5e6...`: 778.37 fps pooled, p99 1.546 ms, three owner-attended cold runs). The H-06 table is still open.
- `--feature` accepts only `none`. `renderer_gate.py` groups by (candidate, scene) only. A runtime option such as `--msaa 4` does not change the build identity, so such runs would be pooled with baseline runs.

## 3. Environment and versions
- Windows 11, RTX 4070 Laptop GPU with 8 GB, NVIDIA driver 616.92, 2560 x 1600 at 60 Hz.
- PresentMon 2.6.0.0, which needs an administrator PowerShell.
- CPython 3.14.7, MSVC with the Windows SDK and DXC.
- The review sandbox is read-only, with no GPU. Evidence kind: source and plan.

## 4. Necessary source and evidence
- The two implementation packets named in section 1.
- `docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md`: item 5 and the time box.
- `docs/progress/1.0/packets/renderer/E-2.4-05-selection.md`: how H-06 is used.
- `docs/wiki/decisions/renderer-candidates.md`: the gate.
- `tools/perf/renderer_gate.py` and `tools/perf/b412_summary.py`.
- `work/experiments/renderer-sb/probe/src/gpu.cpp`, `src/probe.cpp` and `shaders/*.hlsl`.
- Mesh facts measured on the assets are in H6-P section 4. They include 3,948 zero-area and 761 duplicate triangles in the base cell's 10,160, and the feature-edge counts.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | a bare D3D12 draw meets the gate in W3 | E-2.4-01 probe | three owner-attended cold runs | met |
| 2 | the table can be filled from separate gate calls per feature | none | reading `summarize` | would work only if the operator never mixes runs; the grouping makes it safe |

## 6. Constraints and owned files
- Read-only plan check. Under the risk tiers of 2 October 2026, the probe and the tools are non-critical code. The plan check is required because the gate's grouping and the `run.json` contract change. The finished candidate gets one Sol fast review.
- Decided, and out of scope:
  - the seven features and their names;
  - full resolution and every sticker;
  - formal captures are owner-attended with operator confirmation (Astra ruling `20261003T033021Z-274f20af`);
  - unattended runs are preliminary;
  - the baseline and the label path stay unchanged;
  - the order of implementation and the "not implemented" fallback when time runs out;
  - snapshots stay private and are never committed.
- Measurements happen after integration. Claude makes preliminary self-timed runs, and the owner makes formal PresentMon runs.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
