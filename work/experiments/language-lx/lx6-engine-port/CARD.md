# LX-6: Rust hot-path experiment

| Field | Frozen card |
| --- | --- |
| LX card / experiment | LX-6, RTE.56 to RTE.58 / E-2.4-L6 |
| Question | Does the scoped engine hot path match the 0.4 oracle, and what is its cost against NumPy on the owner's machine? |
| Decision it feeds | LQ8 with LX-5; candidate leg of LX-9. Every decision stays **open**, decider: owner. |
| Hypothesis | The Rust proposal lists potential benefits and costs; none is assumed measured. |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; this isolated implementation packet. |
| Time box | what can be done now; one wrapper call of at most two hours |
| Candidate | Base `1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff`; source digests and compiler flags in `CANDIDATE.json`. |
| Method | Export raw model arrays with Python stdlib; Rust validates and replays chronological primitives; compare O01 to O07 at every required step through `tools/oracle/compare.py`. |
| Measurement | Identical frozen inputs against CPython 3.14.7 / NumPy 2.3.5: one smoke, then three series of 100 samples per metric/backend. Full mode is not run here. |
| Kill criteria | Any oracle difference stops the spike until explained; retain first case, step and differing digests. A timeout leaves a partial port, never an accepted engine. A timing digest failure invalidates that run. |
| Evidence class | Source for oracle equality; synthetic negative controls and validation boundaries. Sandboxed smoke timing supplies no performance conclusion. |
| Cannot show | Whole-engine correctness, persistence or input correctness, end-to-end E2, or a language choice from a speed threshold. |
| Product left behind | Scoped Rust port, raw-input digest manifest, trace digests and comparison report, timing harness, smoke observations and this record. |

`INPUTS.json` freezes every named input/source used by the experiment, including the seven compressed case bytes,
`core.py`, `session.py`, comparator, reference generator and replay, model assets, card specification and allowance.
The oracle manifest SHA-256 is `eba3c43f476396ecebee1f3c28dff0e4fee68e131fe7ef278e69734205343612`.
The exporter checks those digests and every asset listed by the unchanged model manifest before running.
It never regenerates a case. All 259,800 slots and all 1,200 legal generators remain present.

No trace fields are omitted. These frozen cases declare no `optional_fields`; the driver rejects omissions even if a
future case declares them. State, net and expansion use little-endian signed int32 bytes. Progress uses the sorted,
compact, ASCII-escaped JSON of the complete 35-row reference progress object, SHA-256 in Rust.
Only provenance fields (`candidate.name`, `candidate.build`) are outside comparator equality; no timing, host or path
fields enter mechanical digests. No personal state or session is read.

The binary export is `LX6MOD1\0`, a u32 byte count plus UTF-8 census/model metadata, then length-prefixed int32le
vectors: slot_piece, orbit_id, offsets, src, dst; u32 tree count; seed, relocation, parent and move for each tree.
The export does not precompute nets, expansions, supports, progress or expected outputs. It is removed after each run.

Commands (from the repository root):

```text
python work/experiments/language-lx/lx6-engine-port/check.py
python work/experiments/language-lx/lx6-engine-port/run_oracle.py
python work/experiments/language-lx/lx6-engine-port/measure.py --smoke
python work/experiments/language-lx/lx6-engine-port/measure.py --conditions work/experiments/language-lx/lx6-engine-port/conditions.json
```

Full mode requires an integrator-created `conditions.json` recording `power_source`, `windows_power_mode`,
`vendor_performance_mode`, `driver_vsync`, `overlays`, `frame_generation`, `cpu`, `gpu`, `quiet_machine` and
`evidence_class`. These are observations to fill on the quiet owner's machine, not assumed settings.
Set `LX6_ENGINE_PYTHON` to the pinned executable only if the root or packet-relative engine location is unavailable.
No installation or network access occurs. Full samples and per-series median/p99 go to `MEASUREMENTS.json`;
p99 is nearest rank. Raw traces are temporary; `ORACLE_RESULTS.json` preserves their hashes and byte counts.

Timing boundaries:

| Metric | Same frozen workload in both backends | Timed work |
| --- | --- | --- |
| apply_generator | O01 step 0's generator on O03's 32-move initial scramble | Lookup plus simultaneous gather/scatter |
| apply_long_word | The 500 O02 generators, in order, on that same initial scramble | Primitive lookup and gather/scatter for all 500 |
| compose_macros | The 25 O04 recipes, each precomposed outside timing | Identity allocation, chronological composition of all 25 nets, sorted net extraction |
| net_effect | O04 step 0's mixed recipe | Normalize and derive its sorted net |

Inputs, loading, per-sample label reset, output hashing, JSON and process startup are outside timing. Rust includes
scratch allocation; NumPy includes its advanced-indexing temporary. No state refresh is timed. Both compare output
digests after every sample; the driver compares backend digests. Each series launches fresh workers and alternates
backend order. NumPy uses `Model.net` with its star cache warmed during preparation; Rust replays primitives without a
star cache. That algorithm difference is intentional and must accompany any future timing interpretation. This is a
scoped candidate comparison, not a claim that the two algorithms or their cache costs are identical.
