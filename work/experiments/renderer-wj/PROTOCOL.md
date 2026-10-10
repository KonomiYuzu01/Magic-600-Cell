# W-J trace and revision-check formats

State input is `magic600-jumbling-render/1`, dimension 4 only, as specified in `research/jumbling/render-contract.md`. All input arrays are checked against the fixture's stage descriptions and their export headers before GPU startup. Unsupported dimensions, invalid counts, stale identities, nonfinite matrices and digest mismatches are refused.

`trace.jsonl` has one entry per drawn captured frame:

```json
{"frame":0,"qpc":123456000000,"turn":0,"phase":0,"revision":0,"camera":1}
```

`qpc` is read after the preceding Present and the frame-context fence wait, before this frame's uploads/draws/Present. `turn = floor((qpc-T0)/D)` and `phase` is the remaining fraction; `D = round(turn_ms*qpc_frequency/1000)`. Angle is `(+/-) angle * phase^2 * (3-2*phase)`; even turns are forward, odd inverse. `revision` is the revision last uploaded to the bundle this frame binds, from the CPU's upload record, not the intended revision; a stale binding therefore shows the older revision. Every ordinary run has `revision == turn`. `camera` counts per-frame rotations, including preroll frames. Trace start/stop markers and QPC frequency are in `run.json`.

`run.json` preserves `magic600-renderer-run-v1`, `candidate: s-b`, the S-B environment/display/window/build/PresentMon fields, `turn_ms` and exact `label_check`. W-J adds:

```json
{
  "scene":"wj",
  "fixture":"wj-s4",
  "overlay":"none",
  "pose_check":{"status":"pass","revisions":[0,1,2]},
  "pose_upload":"whole index, matrix table and lattice table; atomic fenced bundle"
}
```

Fixture names are `wj-s4`, `wj-i-a`, `wj-i-b`. `pose_check.revisions` is sorted and distinct, and contains revisions whose copies matched all three arrays by SHA-256 and element by element. Status is pass only if it equals the required first-use set and all upload/binding/frame checks pass. The separate judge additionally requires that list equal exactly the set of drawn revisions in the marked trace. Failed injections never count as passing runs. The S-B label path is still `scene: w3` and uses engine-side retained labels; it has no pose check.

`pose_check.json` retains the detailed after-capture checks. `required` is the sorted set of required revisions from frame-use records; `revisions` is the matched set. Each `checks` entry gives the copy's `revision`, `frame`, bundle `resource` (1–3), logical `pose_count`, status, and `arrays` in the fixed order pose index / matrix table / lattice table. Each array records its actual and expected SHA-256, `sha256_equal`, `element_equal`, and `element_mismatches`. Integers and float32 bits are compared in groups of four bytes. No float tolerance is applied to adoption.

The CPU records each bundle upload, every frame's intended/actual binding and each first-use copy. It rejects:

- any drawn frame whose actual revision or resource differs from the intended one;
- any required revision without exactly one upload and exactly one first-use copy;
- a first-use frame different from its upload frame, or a copy frame different from first use;
- a copied resource different from the actual draw binding or the intended uploaded resource;
- any count, SHA-256 or element mismatch, an extra/duplicate copy or an injection not reached.

Copy source resources come directly from `drawBinding`, populated when root SRVs are set for the draws. The same command list copies after the final draw and returns the three default resources to shader-read state before submission. Two frame contexts wait for fences before reusing upload bytes; triple bundles are updated in chronological direct-queue order. Readback slots are never recycled within a capture. Tables have a checked logical count and zero-padded spare capacity; the revision check covers all logical entries, including unused poses present in the J1 table, not only poses sampled by the draw.

`geometry_check.json` reports all 18 W-J reference projections per fixture, sample count, maximum absolute error, failures and the full-draw invocation-count check. Geometry tolerance is `abs(GPU-ref) <= 1e-4 + 1e-4*abs(ref)` in NDC x/y and clip w. The source reference emits little-endian float32 triples and a sorted little-endian uint32 sample stream.

`lattice_check.json` reports both controls at all 259,800 slots: exact label failures, shrink-centre failures, geometry-bound failures and off-lattice failures. It uses a 2e-6 world-coordinate tolerance. Unequal bounds disprove geometry agreement; equal bounds alone would not establish it. The retained control has a known source counterexample documented in README and must remain failed until an authorised pipeline/asset correction is accepted.
