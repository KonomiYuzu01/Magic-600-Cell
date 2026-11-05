# E-2.4-L3 part B, step 0: shared bench and negative controls

- LX card: **LX-3**, chapter 11 RTE.44 to RTE.46; CT-RTE-05. `CARD.md`
  retains the card, limits and excluded comparison fields.
- Experiment ID: **E-2.4-L3, part B, step 0**.
- Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2,
  and the assigned shared-bench implementation packet. No installation or cost.
- Time box: **what can be done now; one wrapper call of at most two hours**.
  A reached limit is no pass.
- Frozen candidate identities: **no route candidate**. Negative controls only:
  `control-open` and `control-closed`. Their bindings pin their source, rendered
  notation, dependency bytes, execution/charge mapping, shared kernel/checkers,
  grants and source maps. Raw `bindings.json` SHA-256:
  - control-open: `e96c963fb986f6a63dbc7882dfc29f52b301d38ea2d711bbb2907110ddd845f0`
  - control-closed: `62f9ee6b8b8a16c2e11cd7d2697d525fcc7cdccf2ea7c5d2bbf11967b1928ba8`
- Frozen input digests: corpus **lx3-neutral-v2**; raw `corpus/FREEZE.json`
  SHA-256 `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`.
  The corpus's input/source pins are unchanged. `MANIFEST.json` pins all retained
  part-B files except itself; acceptance also snapshots the manifest itself.
- Commands and tools: below; **CPython 3.15.1**, Python standard library only, on
  Windows. No native compiler, engine, third-party module or external service.
- Evidence class: **source and synthetic**. Child-process death and watchdog
  behavior were exercised on this Windows host; this is no product containment,
  engine, Windows/DirectX or performance result.
- Kill criteria: missing keys/bindings/evidence, changed freeze/source pins,
  reproducibility mismatches, planted access, an unrefused prohibited stimulus,
  wrong refusal code, state/provenance changes, crash or hang fail the relevant
  observation. False refusals are listed without a threshold. A late denial
  cannot erase a forbidden preview or executed unapproved bytes.
- Decision: **open**. Decider: **owner**. LQ2, LQ9, LQ21, BS1 to BS4 and LP1
  metering stay open. No route, engine language, syntax or product policy is chosen.

## Actual result

The shared modules' self-tests passed. The reference executes all frozen
instructions and kernel rows; its policy/approval enforcement is off. The common
meter remains active to bound the recursive probe. Checks cover chronological
permutations/protection, immutable values, retained provenance, receipt opacity,
all allowed values/display plans, four budget counters and interacting limits,
recursion frames, equal-limit admission, ordered cancellation, canonical JSON,
integer precision, display reference/pairwise tolerance, omitted results and
late refusals. The runner separately demonstrated a crash and a watchdog hang.

Each control ran once per case/form/variant/host-cache condition, in fresh child
processes: **480 child invocations and 522 neutral-key observations per control**.
The 82 neutral keys include sequential runs; B12 adds five cache conditions to
its one key. Three forms crossed with two variants remain separately reported.
These are controls under host conditions, not implementations of those forms.

| Observation, in every form/variant | control-open | control-closed |
| --- | --- | --- |
| Must-refuse cases | All 20 fail as unrefused | 19 expected-code refusals; MR08 has the wrong code |
| Allowed tools, denominator 35 | All 35 return the frozen value/observations | All 35 are false refusals, each `LG3:capability.not_granted` |
| Planted resources | All four traps called; planted access detected | No traps called; clock/file/network refusals have the wrong code; process has the expected code |
| Budgets/cache/interruption | All executable cases match; B10 is `not_run: corpus_issue` | Generic refusal is the wrong budget outcome |
| Approved-subset probes | S01 attempt and S02 changed dependency are unrefused | S01 has the expected code; S02 has the wrong code |

Approval probes are `not_run: variant_not_applicable` under `source_bound`, as
their frozen `variants` field requires. No key is removed. B10 is retained as
described in `CORPUS-ISSUES.md`; no stimulus or expected result was repaired.

The loop interruption stops at `r1/body/0:loop_continuation/1`, with used
steps/kernel/memory/output **3/0/1/0**. Batch interruption stops before chunk 1,
with **1/2/117/0**, retaining exactly the completed first chunk. These are logical
fixture charges, not physical memory or elapsed-time measurements. All six cache
conditions preserve the exact payload and **3/6/14/7** counters. Poisoning emits
`LG4:input.cache_entry_mismatch` only in the host envelope. Shared runs hold the
same immutable cached value in overlapping independent invocations; relocated
uses captured bytes restored into another in-memory lookup. This exercises the
synthetic cache contract, not an on-disk product cache or its performance.

B11 actually terminates each open-control child after its admission handshake.
The parent observes the termination and reports `KN7:resource_limit.runtime_lost`.
The shell checkpoint is unchanged, and no program instruction has run. The
closed control refuses before admission, so it does not establish this behavior.

Per-cell refusal lists, every false refusal/code, exact hashes, expanded run keys,
budget/cache/interruption outcomes and reproducibility counterexamples are in
`summaries/control-open.json` and `summaries/control-closed.json`. There were no
cache-comparison counterexamples. **No twenty-run Windows/Linux series ran**;
one observation per key is not reproducibility evidence. Large raw JSONL is
regenerated under `build/` and removed; it is not retained in the patch.

## Production and acceptance

The control binding digests above and `MANIFEST.json` identify the exact retained
source/artifacts. Initial production, run here:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/prepare.py
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/prepare.py --seal
```

`prepare.py` is explicit artifact preparation, never part of acceptance. It
mechanically renders/pins both controls, runs them, stores summaries, and removes
its build outputs. `--seal` only pins the finished files. Do not use preparation
to overwrite unexpected evidence during verification.

Acceptance command, from the repository root:

```text
python work/experiments/language-lx/lx3-sandbox-flow/part-b/check.py
```

The finish condition is **two consecutive exit-0 invocations** on the final
files. The command checks the corpus, its raw freeze pin, the artifact manifest,
every harness module's self-tests, and fresh controls against the stored
summaries. It deletes `build/` even after failure and compares every file's
starting/ending SHA-256, including missing/new files. It prints its acceptance
result; it never refreshes this report, a pin, a summary, timing or evidence.

## Open and not run here

Headless Linux CPython 3.14 validation is **not run here**. The integrator's exact
command, from the repository root on that host, is:

```text
python3.14 work/experiments/language-lx/lx3-sandbox-flow/part-b/check.py
```

The independent integration review remains with the integrator; this packet's
no-network, one-folder boundary excludes nested review-wrapper calls. No commit,
index/ref change, Git configuration change, network access or installation was
performed. Real candidates and their twenty-run series belong to later packets;
the runnable interface and commands are in `harness/README.md`.

B10's trigger remains an integrator corpus-version decision. The bench does not
close any flow-policy gap or implement a stop-sensitive fix. Its records cannot
show **all-program safety, authorship, understanding, absence of covert channels,
product latency or an acceptable false-refusal threshold** (the card's Cannot
show row). They also cannot establish exact cross-platform execution, real engine
correctness, OS sandbox containment, native input, DirectX or long-session behavior.
