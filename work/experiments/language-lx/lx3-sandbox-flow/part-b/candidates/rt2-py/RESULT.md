# LX-3 part B: rt2-py, restricted CPython stand-in

- LX card: **LX-3**, chapter 11 RTE.11, RTE.12, RTE.24 to RTE.27,
  RTE.30 to RTE.37 and RTE.44 to RTE.46; record fields CT-RTE-05.
- Experiment ID: **E-2.4-L3, part B, candidate rt2-py**.
- Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`,
  decision 2, and this candidate's isolated implementation packet.
- Time box: **what can be done now; one wrapper call of at most two hours**.
  A reached limit is no pass.
- Base source: `b91fd20812783dfcbc439c1f4c79b32f6e5eba6c`.
- Frozen inputs: **lx3-neutral-v2**. Raw `corpus/FREEZE.json` SHA-256:
  `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`.
  Its manifest pins every corpus input, including the policy, keys, exact
  profile, fixture, budgets, approval probes and expected codes.
- Frozen candidate: `bindings.json` pins the candidate declaration, renderer,
  checker, bridge, adapter, verification/production sources, all 66 distinct
  rendered Python artifacts, their index, every harness module and the runner.
  `MANIFEST.json` additionally pins all retained evidence and this report.
  Candidate and result digests are recorded with the completed series below.
- Tools: **CPython 3.15.1**, Windows, Python standard library only. No compiler,
  package installation, third-party dependency, network or personal session.
- Evidence class: **source and synthetic**. The Windows child-loss exercise
  uses an actual child termination, but supplies no product containment,
  Windows/DirectX, input, long-session or performance result. Speed is not
  measured in this card.
- Kill criteria: changed input/source pins, missing outcomes, changed run keys
  or exact payloads, wrong codes, unrefused prohibited cases, planted data,
  forbidden state effects, crashes and hangs fail the relevant observation.
  Every false refusal stays listed without a threshold. A refused operation
  cannot erase earlier effects. Declared `not_run` cells are not passes.
- Decision: **open**. Decider: **owner**. No route, engine language, syntax,
  LQ, BS option or LP1 metering contract is selected.

## What this candidate is

**CPython is a stand-in for the embedded-runtime class.** The bench child hosts
unmodified CPython and executes approved Python code objects with domain
bindings. It is not an embedding integration with a separately built VM.
A real embedded candidate such as Luau, Lua, Rhai or a WebAssembly VM needs a
source outside the allowlist and **was not run**.

`render.py` takes only a neutral program. It emits native Python functions,
assignments, branches, loops and module calls, with a byte span for every
neutral instruction, including dormant branches. The output is an experiment
backend representation, never a proposed language syntax or product IR. Loop
locals are renamed so Python's function scope preserves the neutral scope.

`subset.py` admits a closed AST grammar. There are no imports, attribute walks,
subscripts, comprehensions, unrestricted calls, exception handlers, arbitrary
arithmetic or dynamic evaluation. Values are opaque host records; finite domain
operations and their constituent charges use trusted bindings. Before execution
the adapter mechanically renders the actual host stimulus, including substituted
dependencies, and byte-compares the entire program/module closure to its pinned
artifact. It checks, compiles and executes the same AST without a file re-read or
rewrite. Source spans passed to `ctx.executed` come from those actual bytes.

Capability separation scans the AST's transitive call graph. A solving run with
a reachable preview uses the preview-capable grant subset; the shared policy
denies state-reading rows in that run. A run without a preview drops the preview
grant. Non-state fixed-object access remains possible. The bridge also retains
the consumed-state flag and immutable stored provenance. This is a coarse
capability decision, including dormant and indirect preview paths, not proof
that a finer flow mechanism was exercised.

Static labels are computed before execution by AST dataflow: both branches
join, loops reach a conservative fixed point, module results include arguments
and control, and sequential workspace parts retain their labels. Recursive
analysis returns a conservative top summary. The bridge supplies those fixed
annotations and actual host provenance to `ctx.check`; the shared policy alone
decides permission. There is no second neutral interpreter and no per-value
label propagation added to CPython. The existing stop/emission gap is deliberately
unrepaired. Both runtime-label cells are `not_run`: RTE.12/RTE.24 require a
modified VM, and this candidate does not modify one.

## Actual series and six declared cells

Ten Windows repetitions per case/form/variant/cache condition completed: **4,800
fresh child invocations and 5,220 recorded outcomes**. All six cells retain all
87 run/cache identities per repetition. There are **5180 exact payload records**,
whose run keys and payload hashes repeat identically, and **40 recorded crashes**
without payloads. Those crashes are B12/shared in the four declared `not_run`
cells (ten each), caused by the documented harness bookkeeping issue. There
are no hangs. No returned-payload reproducibility counterexample was observed.
The required Windows/Linux twenty-run series is **not run here**.

- Raw `bindings.json` SHA-256: `77c962966daa2054be7b1647d884a33f2e43dd516ffb8168df8ddf5568830755`.
- Raw `summary.json` SHA-256: `704ccd9ddf9c07b8a68eaa76baa3fc6b0bc3e8f104fbee4f9b351cf3465e3889`.
- Raw production `results.jsonl`: **128767130 bytes**, SHA-256
  `f3d7c61de48b45b66d59bae0b3e4f81992ea4099ade73f2bf77489f61fa73426`; removed, reproducible with the recorded production command.

Capability separation refuses all **20/20** must-refuse cases, with **2/35**
false refusals (A25, A30). Static labels refuse **16/20**, with **0/35** false
refusals. **MR13, MR14, MR19 and MR20 fail static labels as unrefused**: the
stop/accumulation channel remains open (eight failing run identities per
repetition, including the sequential MR19/MR20 invocations). These failures
are retained. Both supported cells refuse all four planted probes and both
approval probes with their expected codes. Their 13 executable budget cases,
including all six B12 cache conditions, match the frozen outcomes; B10 does not run.

Tables below aggregate cases over ten repetitions; budget rows show the
repeat-0 outcome, which repeated in the other nine runs. Used counters are
**steps/kernel/memory/output**. Every refusal is listed by case/run and code;
the full key/hash lists, classifications and repeated records are in `summary.json`.

### capability_separation / source_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run: RT2 requires approved_subset_byte_bound; source_bound not implemented |
| Must-refuse passes | Not measured: all 20 cases are not_run; no pass is inferred from the summary's zero failure count. |
| Wrong codes | Not measured; no refusal gate ran. |
| Allowed tools and false refusals | All 35 are not_run; the recorded zero false-refusal count is not an allowed-tool result. |
| Every refused program and code | None; candidate execution was not admitted. |
| Planted reads / approval | P01-P04 and S01-S02 are not_run. No candidate execution; no sandbox claim. |
| Budget and interruption | B01-B09, B11, B13 and B14 are not_run with the declaration reason. |
| B10 | not_run with the declaration reason, rather than corpus_issue; harness early-return limitation, not a pass. |
| B12 cache conditions | cold, warm, evicted, poisoned and relocated: not_run. shared: crash in all ten repetitions (harness bookkeeping). |
| Reproducibility counterexamples | 0 among returned not_run payloads; no runtime behavior was measured. |
| Crashes / hangs | 10 / 0; all crashes are B12/r1/shared; no exact payload hash exists for a crash. |

### capability_separation / approved_subset_byte_bound

| Observation | Result |
| --- | --- |
| Declaration | supported |
| Must-refuse passes | 20/20; no failures |
| Wrong codes (must-refuse/planted/approval) | 0 |
| Allowed tools | 33/35 allowed_ok |
| False refusal count | 2/35; no threshold |
| Each false refusal | A25: `LG3:capability.not_granted`; A30: `LG3:capability.not_granted` |
| Planted reads | 0; P01-P04 all refused_expected, no trap data or forbidden state effects |
| Approved-subset probes | S01: LG3:capability.not_granted; S02: LG5:input.module_digest_mismatch; both refused_expected before unapproved execution |
| Reproducibility counterexamples | 0 across the ten Windows repetitions and comparable cache conditions; Linux not run here |
| Crashes / hangs | 0 / 0; B11 is the expected, separately classified runtime-loss exercise |
| Refused: `KN7:input.budget_not_declared` (layer `rt2_py`) | B13/r1 |
| Refused: `KN7:resource_limit.budget_ceiling` (layer `rt2_py`) | B14/r1 |
| Refused: `LG3:capability.ambient_read` (layer `shared_policy`) | P01/r1, P02/r1, P03/r1 |
| Refused: `LG3:capability.not_granted` (layer `shared_policy`) | A25/r1, A30/r1, MR01/r1, MR02/r1, MR03/r1, MR04/r2, MR05/r1, MR06/r1, MR07/r2, MR09/r1, MR10/r1, MR11/r1, MR12/r1, MR13/r1, MR14/r1, MR15/r1, MR16/r1, MR17/r1, MR18/r1, MR19/r1, MR19/r2, MR19/r3, MR20/r1, MR20/r2, MR20/r3, P04/r1, S01/r1 |
| Refused: `LG5:input.module_digest_mismatch` (layer `rt2_byte_binding`) | S02/r1 |
| Refused: `PS1:capability.provenance_incompatible` (layer `shared_policy`) | MR08/r2 |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0; stop `r1/body/2:before_step` |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0; stop `r1/body/1:before_kernel` |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0; stop `r1/body/1:before_memory` |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6; stop `r1/body/1:before_output` |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0; stop `r1/body/0:before_kernel` |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0; stop `r1/body/0:before_step` |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0; stop `r1/recurse:before_frame_4` |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0; stop `r1/body/0:loop_continuation/1` |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0; stop `r1/body/0:chunk/1` |
| B10/r1/default | not_run: corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0; stop `r1/after_admission` |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0; stop `r1/admission` |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0; stop `r1/admission` |

### static_labels / source_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run: RT2 requires approved_subset_byte_bound; source_bound not implemented |
| Must-refuse passes | Not measured: all 20 cases are not_run; no pass is inferred from the summary's zero failure count. |
| Wrong codes | Not measured; no refusal gate ran. |
| Allowed tools and false refusals | All 35 are not_run; the recorded zero false-refusal count is not an allowed-tool result. |
| Every refused program and code | None; candidate execution was not admitted. |
| Planted reads / approval | P01-P04 and S01-S02 are not_run. No candidate execution; no sandbox claim. |
| Budget and interruption | B01-B09, B11, B13 and B14 are not_run with the declaration reason. |
| B10 | not_run with the declaration reason, rather than corpus_issue; harness early-return limitation, not a pass. |
| B12 cache conditions | cold, warm, evicted, poisoned and relocated: not_run. shared: crash in all ten repetitions (harness bookkeeping). |
| Reproducibility counterexamples | 0 among returned not_run payloads; no runtime behavior was measured. |
| Crashes / hangs | 10 / 0; all crashes are B12/r1/shared; no exact payload hash exists for a crash. |

### static_labels / approved_subset_byte_bound

| Observation | Result |
| --- | --- |
| Declaration | supported |
| Must-refuse passes | 16/20; unrefused: MR13, MR14, MR19, MR20 |
| Wrong codes (must-refuse/planted/approval) | 0 |
| Allowed tools | 35/35 allowed_ok |
| False refusal count | 0/35; no threshold |
| Each false refusal | None |
| Planted reads | 0; P01-P04 all refused_expected, no trap data or forbidden state effects |
| Approved-subset probes | S01: LG3:capability.not_granted; S02: LG5:input.module_digest_mismatch; both refused_expected before unapproved execution |
| Reproducibility counterexamples | 0 across the ten Windows repetitions and comparable cache conditions; Linux not run here |
| Crashes / hangs | 0 / 0; B11 is the expected, separately classified runtime-loss exercise |
| Refused: `KN7:input.budget_not_declared` (layer `rt2_py`) | B13/r1 |
| Refused: `KN7:resource_limit.budget_ceiling` (layer `rt2_py`) | B14/r1 |
| Refused: `LG3:capability.ambient_read` (layer `shared_policy`) | P01/r1, P02/r1, P03/r1 |
| Refused: `LG3:capability.not_granted` (layer `shared_policy`) | MR01/r1, MR02/r1, MR03/r1, MR04/r2, MR05/r1, MR06/r1, MR07/r2, MR09/r1, MR10/r1, MR11/r1, MR12/r1, MR15/r1, MR16/r1, MR17/r1, MR18/r1, P04/r1, S01/r1 |
| Refused: `LG5:input.module_digest_mismatch` (layer `rt2_byte_binding`) | S02/r1 |
| Refused: `PS1:capability.provenance_incompatible` (layer `shared_policy`) | MR08/r2 |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0; stop `r1/body/2:before_step` |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0; stop `r1/body/1:before_kernel` |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0; stop `r1/body/1:before_memory` |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6; stop `r1/body/1:before_output` |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0; stop `r1/body/0:before_kernel` |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0; stop `r1/body/0:before_step` |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0; stop `r1/recurse:before_frame_4` |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0; stop `r1/body/0:loop_continuation/1` |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0; stop `r1/body/0:chunk/1` |
| B10/r1/default | not_run: corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0; stop `r1/after_admission` |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0; stop `r1/admission` |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0; stop `r1/admission` |

### static_runtime_labels / source_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run: RT2 requires approved_subset_byte_bound; source_bound not implemented |
| Must-refuse passes | Not measured: all 20 cases are not_run; no pass is inferred from the summary's zero failure count. |
| Wrong codes | Not measured; no refusal gate ran. |
| Allowed tools and false refusals | All 35 are not_run; the recorded zero false-refusal count is not an allowed-tool result. |
| Every refused program and code | None; candidate execution was not admitted. |
| Planted reads / approval | P01-P04 and S01-S02 are not_run. No candidate execution; no sandbox claim. |
| Budget and interruption | B01-B09, B11, B13 and B14 are not_run with the declaration reason. |
| B10 | not_run with the declaration reason, rather than corpus_issue; harness early-return limitation, not a pass. |
| B12 cache conditions | cold, warm, evicted, poisoned and relocated: not_run. shared: crash in all ten repetitions (harness bookkeeping). |
| Reproducibility counterexamples | 0 among returned not_run payloads; no runtime behavior was measured. |
| Crashes / hangs | 10 / 0; all crashes are B12/r1/shared; no exact payload hash exists for a crash. |

### static_runtime_labels / approved_subset_byte_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run: Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified |
| Must-refuse passes | Not measured: all 20 cases are not_run; no pass is inferred from the summary's zero failure count. |
| Wrong codes | Not measured; no refusal gate ran. |
| Allowed tools and false refusals | All 35 are not_run; the recorded zero false-refusal count is not an allowed-tool result. |
| Every refused program and code | None; candidate execution was not admitted. |
| Planted reads / approval | P01-P04 and S01-S02 are not_run. No candidate execution; no sandbox claim. |
| Budget and interruption | B01-B09, B11, B13 and B14 are not_run with the declaration reason. |
| B10 | not_run with the declaration reason, rather than corpus_issue; harness early-return limitation, not a pass. |
| B12 cache conditions | cold, warm, evicted, poisoned and relocated: not_run. shared: crash in all ten repetitions (harness bookkeeping). |
| Reproducibility counterexamples | 0 among returned not_run payloads; no runtime behavior was measured. |
| Crashes / hangs | 10 / 0; all crashes are B12/r1/shared; no exact payload hash exists for a crash. |

B08 stops at the frozen loop continuation with used counters **3/0/1/0**.
B09 stops before batch chunk 1 with **1/2/117/0**, preserving exactly the
completed first chunk. Every B12 cache condition has **3/6/14/7**; poisoned
cache rejection is `LG4:input.cache_entry_mismatch` in the host envelope only.
B11 kills each supported-cell child after admission (20 actual terminations
across the series), with zero program instructions and an unchanged shell
checkpoint. These observations are logical/synthetic contracts, not physical
allocation, wall-time interruption or performance measurements.

## Candidate-local adversarial escapes

`probes.py` and `probes.json` retain these observations outside the corpus and
its counts. The approved execution path rejects every expression below before
entering the VM. A separate negative control deliberately bypasses the checker
but keeps empty builtins; it only examines object metadata and never invokes
recovered authority. No object contents, machine paths or diagnostics are saved.

| Attempt | AST checker | Approved execution | Runtime with checker deliberately bypassed |
| --- | --- | --- | --- |
| `(1).__class__.__mro__[1].__subclasses__()` | Rejected | Not entered | Returned class-list metadata |
| `h_literal.__globals__` | Rejected | Not entered | Helper globals reachable, including builtins |
| `'{0.__class__.__mro__}'.format(1)` | Rejected | Not entered | Returned type metadata as text |
| `__import__('os')` | Rejected | Not entered | `NameError`, builtin absent |
| `eval('1 + 1')` | Rejected | Not entered | `NameError`, builtin absent |
| `open('candidate-local-unused-file')` | Rejected | Not entered | `NameError`, builtin absent; no file opened |

Empty builtins alone are insufficient for this boundary. The table establishes
only these attempted escapes and the necessity of the checker in this candidate;
it is not an all-program containment proof.

## RTE.11 answers and limits

| Question | Observed answer and boundary |
| --- | --- |
| Reachable host facilities | The approved grammar reaches only its sealed inputs, fake kernel rows, case-local workspace, captured manual picks, bounded output/display and policy-gated preview/capture bindings. The planted clock/file/network/process requests are denied before their synthetic traps. No real OS resource access is tested by those traps. The local negative controls show why exposing ordinary Python objects without the checker would be unsafe. |
| Dynamic code and byte binding | S01 is refused through the shared dynamic-authority policy before compilation/execution; S02's substituted module bytes disagree with the unchanged approval and are refused before any instruction executes. Every executed neutral instruction is mapped to approved Python bytes. These observations cover the frozen closure and substitution, not an arbitrary product loader. |
| Every limit and interrupt | The frozen logical step/kernel/memory/output charges, admission boundaries, recursion frames, cancellation continuations, batch chunks, cache invariance and actual child loss are exercised. B10 gives no wall-clock safety-stop evidence. Physical CPython allocation, AST/compiler resource use, arbitrary native extension interruption and general hostile-program termination are **not known here**. A logical `retain` is not a physical allocation stress test. |
| Exact results repeat | The completed series below compares full run keys and exact payload hashes, including coverage, counters, stop and provenance. Linux and the required twenty-run cross-platform comparison are **not run here**. This says nothing about every Python program or product determinism. |
| Dependency and modification licences | The local `licenses/Python-LICENSE.txt` states PSF License Version 2, retention of the licence and copyright notice (clause 2), and a brief summary of changes for distributed derivatives (clause 3); it also records other incorporated-software terms. The local root `LICENSE` states the project's MIT notice-retention condition. CPython was not modified or redistributed here. The exact installed distribution's complete licence/dependency audit, and obligations for the unrun VMs, are **not known here**. No licence conclusion is taken from memory. |

The source closure pins project and candidate modules. The host CPython binary
and standard library are supplied by the bench environment, identified here by
the observed version; they are not vendored or established as a byte-closed
product runtime. No speed or product sandbox property follows from that version.

The card **cannot show all-program safety, authorship, understanding, absence of
covert channels, product latency or an acceptable false-refusal threshold**.
It also cannot establish real-engine correctness, physical memory containment,
native rendering/input behavior, cross-platform exactness or a modified-VM form.

## Bench issues

1. **B10 remains unresolved evidence, never a pass.** The frozen short loop has
   no sustained wall-clock workload. `part-b/CORPUS-ISSUES.md` records the
   integrator's decision to keep the corpus unchanged and B10
   `not_run: corpus_issue`. The two supported cells reach admission and report
   exactly that reason. In the four declared `not_run` cells,
   `harness/matrix.py` returns the cell's reason before calling the adapter, so
   their B10 records retain the cell-level reason instead. The candidate cannot
   change that reason without changing the harness or misdeclaring a cell.
   Those keys remain `not_run`; the per-cell tables disclose the reporting gap.
2. **B12/shared crashes in declared `not_run` cells.** In
   `harness/matrix.py`, `execute_run` returns early for a `not_run` declaration.
   The outer shared-cache worker still demands
   `record["host"]["shared_value_identity"]`, which that early return does not
   populate. A direct worker reproduction raises
   `KeyError: 'shared_value_identity'`. The runner records `crash`, not a
   fabricated `not_run` or success.
   The two supported cells exercise and pass the shared-cache identity check.
   The four unsupported cells retain their affected B12/shared failure keys;
   no dummy field, manifest exception or harness patch is supplied here.
   Acceptance compares the recorded failure outcomes as well as the hashes
   of records that exist. A crash has no exact payload hash; it never masquerades
   as a successful payload. This is a harness follow-up for the integrator.
3. No corpus expectation, shared harness code or control was changed, and no
   failed case was excused. The static stop/accumulation failures are candidate
   results, not new corpus issues.

## Production, acceptance and remaining commands

Executed from the repository root (the first command produces artifacts;
acceptance never calls it):

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-py/prepare.py --runs 10
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-py/prepare.py --seal
python work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-py/check.py
```

The acceptance command verifies the raw/normalized corpus freeze and every
candidate/artifact pin, regenerates source in memory and byte-compares all
artifacts and source maps, checks the independent local fixtures and escape
table, and runs this one-pass command:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt2-py --out candidates/rt2-py/build/check --runs 1 --leg Windows
```

It verifies all 522 outcome identities and compares exact payload hashes, run
keys and classifications against every retained repetition. Negative form
results do not make acceptance fail. It never refreshes evidence or pins; all
temporary outputs are removed even on failure, and it compares every candidate
file's starting/ending hash, including the manifest. Raw production JSONL above
2 MB is removed; `series.json` retains its byte count, SHA-256 and run identity.
The report is editable Markdown; the exact commands and pinned Python sources
above produce its measured input tables. No external artifact service is used.

**Not run here:** the Linux leg and Windows/Linux combination. The integrator
needs raw folders on the comparison host; regenerate Windows raw data and run
Linux using fresh output directories, from the repository root on each host:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt2-py --out candidates/rt2-py/build/Windows --runs 10 --leg Windows
python3.14 -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt2-py --out candidates/rt2-py/build/Linux --runs 10 --leg headless_Linux
```

With both folders present, from `work/experiments/language-lx/lx3-sandbox-flow/part-b/`:

```text
python -B -m harness.report --inputs candidates/rt2-py/build/Windows candidates/rt2-py/build/Linux --out candidates/rt2-py/build/comparison.json
```

Preserve the comparison evidence separately and remove generated `build/` before
running acceptance again. No GPU or decisive timing command belongs to this
card. Modified-VM forms and real embedded runtimes remain unrun; they need new
scoped work and any applicable owner allowance. Independent integration review
remains with the integrator: this packet forbids the network and out-of-folder
writes needed for a nested review-wrapper call. No commit, staging, ref/config
change, fetch, push, installation or network operation was performed.

## Effort and reuse

The work built a mechanical Python renderer, closed AST grammar, static dataflow
checker, native-control-flow bridge, byte-binding adapter, local escape and
semantic fixtures, and reproducible preparation/acceptance scripts. The shared
policy, exact calculation tariff, meter, synthetic host/kernel, matrix runner
and classifier were reused unchanged. The non-enforcing reference supplied
operation-wiring guidance; it is not the candidate's executor.

This is throwaway experiment code: rendered programs, finite helper surface and
checker annotations establish no product syntax, full type system, saved-tool
compatibility or runtime recommendation. No third-party VM, compiler port,
product module, UI or additional flow repair was built. Work stayed within one
wrapper call and the assigned folder; the exact source and repeated-run evidence
are retained for integrator review. The only planned follow-up measurements are
the unrun cross-platform series above; owner decisions remain open.

Sources read: `AGENTS.md`, `docs/development-guide/AGENT_BRIEFING.md`, the named
chapter-11 clauses and owner allowance, `work/experiments/language-lx/README.md`,
the bench interface and shared modules, the two control formats, the frozen
corpus files, and `part-b/CORPUS-ISSUES.md`. Moving governance/card documents
are named rather than pinned as acceptance dependencies.
