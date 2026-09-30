---
id: build-reproducibility-decisions
type: decision
status: verified
visibility: public
summary: Owner decisions of 30 September 2026 for 0.4.1 step 1: published harness and continuity record, CPython 3.14.7 with NumPy 2.3.5, byte-exact checkouts, optional desktop recorder.
related: [build-identity-v2, owner-decisions-2026-09-29]
supersedes: []
claims:
  - {id: d1-publish-harness, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: d2-engine-python, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: d3-byte-exact, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: d4-optional-recorder, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: harness-runner, evidence_kind: source, path: work/experiments/magic600-04/tests/run_postapproval.py, sha256: 0616a958e92b589ccdadad427083634a4f5b4f3fe606bdb59c7138ad6cf3ecd2, checked_at: 2026-09-30}
  - {id: recorder-skip, evidence_kind: source, path: work/experiments/magic600-04/tests/PostApprovalNativeRegression.cs, sha256: 421f1d3cafc420fb7aaecc2a90ab451c647b112ad6cd325174f5cb4b45478904, checked_at: 2026-09-30}
  - {id: checkout-repair, evidence_kind: source, path: tools/checkout_bytes.py, sha256: 9b8917567713e9dd739aebf7b6ef3fe2c4d5bb980adde86c60d8005d1f3c892f, checked_at: 2026-09-30}
  - {id: continuity-tool, evidence_kind: source, path: tools/provenance/continuity_04.py, sha256: 2f02438d214d80930cf735eaa120f15928601059e13340f261e1212ebb71d616, checked_at: 2026-09-30}
---

# Build reproducibility decisions, 30 September 2026

The owner made four decisions for 0.4.1 step 1, which delivers a reproducible identity and harness. They followed the Codex-checked plan, and the resulting design is described in [build-identity-v2](../concepts/build-identity-v2.md).

- **D1: publish the harness and a sanitized continuity record.** The 0.4 native harness (`work/experiments/magic600-04/tests/`: the runner, the C# checks and the workflow cases) is published byte-exact. `docs/RELEASE_0_4_CONTINUITY.json` maps the 0.4 release receipt onto the tree. It contains no private paths, no machine names and no Windows build number.
- **D2: CPython 3.14.7 with NumPy 2.3.5.** The engine environment is the allowlisted `engine-python` entry. It uses the owner-installed CPython 3.14.7 and hash-pinned NumPy 2.3.5, never a downloaded or uv-managed interpreter. The 0.4 release used CPython 3.12.14, so its build is linked by the continuity record, not rebuilt.
- **D3: byte-exact checkouts repository-wide.** `.gitattributes` sets `* -text`, so every file keeps its committed bytes, including the CRLF and mixed-ending files. `tools/checkout_bytes.py --fix` repairs a working tree converted before the rule. It never stages or deletes anything, and moves each original into a backup under the Git directory.
- **D4: the desktop recorder is optional.** Actual desktop frames are captured only when `run_postapproval.py --recorder <ffmpeg.exe>` is given. Only the recorder's hash is recorded, never its path. Without a recorder, each frame check is logged as `SKIP` and does not count as a pass. No recorder binary is published.
