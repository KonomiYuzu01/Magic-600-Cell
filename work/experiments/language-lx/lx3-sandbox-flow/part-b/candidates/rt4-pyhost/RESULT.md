# E-2.4-L3 part B: rt4-pyhost

This candidate tests RT4's host-language-library mechanism. **Python stands in
for a host; a real RT4 host would be chosen under LQ8. These results say something
about the mechanism, never about which host to choose.** This is experiment code,
not product code, a syntax proposal, route admission or an answer to any LQ.

## Record and scope

- LX card: **LX-3**, chapter 11 RTE.44 to RTE.46; CT-RTE-05.
- Experiment ID: **E-2.4-L3, part B, candidate rt4-pyhost**; bench interface v1.
- Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2,
  and the assigned LX-3 part-B co-implementation packet, base `b91fd208`.
- Time box: **what can be done now; one wrapper call of at most two hours**.
  A reached limit is no pass.
- Inputs: frozen **lx3-neutral-v2**. Raw `corpus/FREEZE.json` SHA-256:
  `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`.
  Its content digest is
  `c4aa5b09430d8e5dbe02c63686b4fb400b36e7e7faf82dc42113491efe1e28c7`.
- Candidate identity: `bindings.json` pins candidate sources, all shared harness
  modules, source maps, rendered Python bytes, the execution contract and meter
  mapping. `MANIFEST.json` additionally pins the report, summary and observer
  evidence. Exact candidate digests are recorded with the finished series below.
- Tools: **CPython 3.15.1**, Python standard library only, Windows 11. No engine,
  NumPy, Node, compiler, installed package or external service is used.
- Evidence class: **source and synthetic**. Even the observed Windows child
  termination uses fake data; this is no Windows/DirectX or OS containment result.
- Kill criteria: an unrefused prohibited case, wrong code, changed protected
  state, planted read, unapproved execution, reproducibility mismatch, crash or
  hang fails the relevant observation. A refusal counts only with the frozen
  code and preservation checks. False refusals retain the full denominator 35
  without a threshold. Missing outcomes and changed pins fail acceptance.
- Decision: **open**. Decider: **owner**. LQ2, LQ8, LQ9, LQ13, LQ21, BS1 to BS4,
  LP1 metering and every other language question remain open.

## Mechanism

`render.py::render(program)` accepts only the neutral program. It cannot see a
case ID, family, expected code or expected result. The trusted artifact assembler
concatenates its Python source into `programs.py`; `program-index.json` selects
each independent source slice. Source maps include both arms and every module.
The adapter re-renders the actual stimulus, compares the whole slice with the
pinned approval, checks that same byte sequence's AST and compiles that sequence.
S02 therefore compares the substituted return-2 dependency with the retained
return-1 approval before executing either. The adapter uses a case ID only to
locate its approved slice, never to select mechanism behavior.

Programs use Python `if`, `for`, functions and returns. Kernel operations,
workspace, picks, preview, display, exact calculations and budget effects go
through the domain `Library` object. The library holds the effective grants.
`ctx.check` supplies all policy decisions. The mechanism reads no answer data.
The approved subset excludes imports, ambient built-ins, reflection, replacement
of the library object and escaping method values. Dynamic evaluation is refused
by shared policy during approval; the host never evaluates its text.

- **Capability separation:** scan the active function's transitive call closure
  for preview use, then freeze exclusive read-or-preview grants in solving.
  The library retains the run's observed read-state bit and stored provenance.
  The coarse grant conflict also applies to independent facts and manual previews.
- **Static labels:** abstract interpretation of the approved Python AST joins
  both branches, loop assignments and module results. It prechecks the inferred
  effects through shared policy. Persisted provenance is a symbolic input,
  instantiated and checked at the library boundary. The host builds blocks from
  actual data and available control dependencies. This finite corpus checker
  supplies no general recursive-effect theorem and no stop-sensitive repair.
- **Additional run-time labels, wrapper-only attempt:** structural subset
  approval and explicit-value propagation run, but the static flow checker is
  not enabled in this attempt. Native Python truth testing returns a Boolean
  without recording a control label. Meter callbacks identify charged
  instructions; they do not add branch dependencies to wrapper values. This is
  deliberately an incomplete attempt at the prescribed form, not a claim that
  wrappers implement the RTE.24 runtime modification. Its escapes count as failures.

The library reuses the shared exact arithmetic tariff, meter, kernel, cache,
host provenance and policy. Batch storage is charged only by the shared runtime;
other bindings, parameters, loop items and recursive frames retain the neutral
charges. No generic neutral instruction evaluator is used to execute programs.
The trusted adapter and empty Python built-ins are not an OS security boundary.

## Actual result

The Windows series completed **ten repetitions per observation identity**, with
**4,800 fresh child invocations and 5,220 recorded observations** across all six
declared cells. This includes 5,190 exact payload records and 30 explicit bench
crashes at three skipped-cell B12/shared identities. No observation is omitted.
All available repeated payloads have one full run key and one exact SHA-256;
all three supported cells also agree across all six cache conditions. There are
no reported reproducibility counterexamples or hangs. The twenty-run
Windows/Linux series is **not complete** because Linux is not run here.

- Raw `bindings.json` SHA-256: `b036e0991cc9cc0d173282e6bab7eeabfe3ee4a9c488ac08746abe3e54b50a16`. 
- Raw `summary.json` SHA-256: `23b4c743a901d17e9fcee7437e37b24e6aed0c843a561a9a3f37ba21e04d9cce`. 
- Raw `evidence.json` SHA-256: `b8ccf8ad87863eacbdd210fac5fc71f082eba5e7f80628e094b5c0e6b2f04ae4`. 
- Removed raw `results.jsonl`: **59,279,440** bytes; SHA-256
  `b6410442cad02c734cfba0d52b5d8d43e74f651b86f644d0c5522e701406e8b7`. The production command below regenerates it.

`summary.json` is the unchanged shared-bench summary: every per-cell outcome,
refusal/code, repeated key/hash and budget/cache/interruption observation.
`evidence.json` retains small first-repetition observer records, including actual
gate paths, before/after proposal words and targets, session preservation, trap
calls, approval execution counts and cache diagnostics. Repeats are represented
individually in the summary; the following counts are cases or named conditions,
not tenfold case counts. All refusal rows list the actual gate layer.

Capability separation refused all 20 prohibited cases, but falsely refused A25
and A30: each combines an independent facts report with a manual/declassified
preview. Those two refusals remain in the 35-tool denominator. In MR13 the
library can install the earlier permitted manual preview before refusing the
score read; the bench accepts that gate because no state read selected the
proposal. This is the coarse capability boundary, not evidence that inner
flow-label or stop-sensitive machinery ran.

Static labels refused 16/20 cases. MR13, MR14, MR19 and MR20 remain unrefused:
ordinary branch joins do not repair stop-selected emission or accumulation.
The wrapper-only attempt refused 10/20. Its additional misses are MR09 to MR12
and MR15/MR16. In MR09 the native state-conditioned branch installs word `[a]`
at target `c0`; in MR10 the branch assignment installs `[b]` at `c0`. Both carry
clean constant-word wrappers to the preview despite a state-derived condition.
MR11/MR12 carry the same loss through native module calls, and MR15/MR16 use
preview outcomes to select the next proposal. These are observed counterexamples,
not exemptions, alternate stimuli or proposals to instrument host branches.

In each supported cell B08 stops before loop continuation 1 with steps/kernel/
memory/output **3/0/1/0**; B09 stops before batch chunk 1 with **1/2/117/0** and
one completed chunk. B11 really terminates the admitted child with zero charged
work and unchanged shell checkpoint. B12 has **3/6/14/7** in all six conditions;
the poisoned cache emits `LG4:input.cache_entry_mismatch` only in host diagnostics.
These are synthetic logical charges and cache-validation results, not timings.

### capability_separation / source_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run; RT4 attempt requires approved host subset and executed-byte binding; source_bound is not implemented |
| Must-refuse passes | 0/20; all 20 unrun, no enforcement evidence |
| Must-refuse failures | No executable result |
| Wrong registry codes | 0 recorded; unrun |
| Allowed tools | 0/35 allowed_ok |
| False refusals | 0/35; all allowed tools unrun |
| Planted reads | P01, P02, P03, P04 not_run; no containment evidence |
| Approval | S01 and S02 not_run |
| Reproducibility counterexamples | 0; 86 skipped identities with payloads; crash identity has no payload |
| Crashes / hangs | 10 / 0; B12/r1/shared in all ten repetitions, bench issue below |
| B01/r1/default | not_run; cell declaration |
| B02/r1/default | not_run; cell declaration |
| B03/r1/default | not_run; cell declaration |
| B04/r1/default | not_run; cell declaration |
| B05/r1/default | not_run; cell declaration |
| B06/r1/default | not_run; cell declaration |
| B07/r1/default | not_run; cell declaration |
| B08/r1/default | not_run; cell declaration |
| B09/r1/default | not_run; cell declaration |
| B10/r1/default | not_run; cell declaration |
| B11/r1/default | not_run; cell declaration |
| B12/r1/cold | not_run; cell declaration |
| B12/r1/warm | not_run; cell declaration |
| B12/r1/evicted | not_run; cell declaration |
| B12/r1/poisoned | not_run; cell declaration |
| B12/r1/shared | crash |
| B12/r1/relocated | not_run; cell declaration |
| B13/r1/default | not_run; cell declaration |
| B14/r1/default | not_run; cell declaration |
| Refused programs and codes | None; candidate did not execute |
| Decision / decider | open / owner |

### capability_separation / approved_subset_byte_bound

| Observation | Result |
| --- | --- |
| Declaration | supported; experiment attempt |
| Must-refuse passes | 20/20 |
| Must-refuse failures | None observed |
| Wrong registry codes | 0 |
| Allowed tools | 33/35 allowed_ok |
| False refusals | 2/35; A25: `LG3:capability.not_granted`; A30: `LG3:capability.not_granted` |
| Planted reads | 0; P01, P02, P03, P04 each refused_expected; zero trap calls |
| Approval | S01 and S02 refused_expected; zero dynamic executions and zero unapproved fragments |
| Reproducibility counterexamples | 0; 87 observation identities with exact payloads |
| Crashes / hangs | 0 / 0 |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0 |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0 |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0 |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6 |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0 |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0 |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0 |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0 |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0 |
| B10/r1/default | not_run; corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0 |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0 |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0 |
| Refused A25/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused A30/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused B13/r1 | `KN7:input.budget_not_declared`; rt4_library |
| Refused B14/r1 | `KN7:resource_limit.budget_ceiling`; rt4_library |
| Refused MR01/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR02/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR03/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR04/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR05/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR06/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR07/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR08/r2 | `PS1:capability.provenance_incompatible`; shared_policy |
| Refused MR09/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR10/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR11/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR12/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR13/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR14/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR15/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR16/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR17/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR18/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR19/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR19/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR19/r3 | `LG3:capability.not_granted`; shared_policy |
| Refused MR20/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR20/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR20/r3 | `LG3:capability.not_granted`; shared_policy |
| Refused P01/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P02/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P03/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P04/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused S01/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused S02/r1 | `LG5:input.module_digest_mismatch`; byte_approval |
| Decision / decider | open / owner |

### static_labels / source_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run; RT4 attempt requires approved host subset and executed-byte binding; source_bound is not implemented |
| Must-refuse passes | 0/20; all 20 unrun, no enforcement evidence |
| Must-refuse failures | No executable result |
| Wrong registry codes | 0 recorded; unrun |
| Allowed tools | 0/35 allowed_ok |
| False refusals | 0/35; all allowed tools unrun |
| Planted reads | P01, P02, P03, P04 not_run; no containment evidence |
| Approval | S01 and S02 not_run |
| Reproducibility counterexamples | 0; 86 skipped identities with payloads; crash identity has no payload |
| Crashes / hangs | 10 / 0; B12/r1/shared in all ten repetitions, bench issue below |
| B01/r1/default | not_run; cell declaration |
| B02/r1/default | not_run; cell declaration |
| B03/r1/default | not_run; cell declaration |
| B04/r1/default | not_run; cell declaration |
| B05/r1/default | not_run; cell declaration |
| B06/r1/default | not_run; cell declaration |
| B07/r1/default | not_run; cell declaration |
| B08/r1/default | not_run; cell declaration |
| B09/r1/default | not_run; cell declaration |
| B10/r1/default | not_run; cell declaration |
| B11/r1/default | not_run; cell declaration |
| B12/r1/cold | not_run; cell declaration |
| B12/r1/warm | not_run; cell declaration |
| B12/r1/evicted | not_run; cell declaration |
| B12/r1/poisoned | not_run; cell declaration |
| B12/r1/shared | crash |
| B12/r1/relocated | not_run; cell declaration |
| B13/r1/default | not_run; cell declaration |
| B14/r1/default | not_run; cell declaration |
| Refused programs and codes | None; candidate did not execute |
| Decision / decider | open / owner |

### static_labels / approved_subset_byte_bound

| Observation | Result |
| --- | --- |
| Declaration | supported; experiment attempt |
| Must-refuse passes | 16/20 |
| Must-refuse failures | MR13, MR14, MR19, MR20 |
| Wrong registry codes | 0 |
| Allowed tools | 35/35 allowed_ok |
| False refusals | 0/35; none |
| Planted reads | 0; P01, P02, P03, P04 each refused_expected; zero trap calls |
| Approval | S01 and S02 refused_expected; zero dynamic executions and zero unapproved fragments |
| Reproducibility counterexamples | 0; 87 observation identities with exact payloads |
| Crashes / hangs | 0 / 0 |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0 |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0 |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0 |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6 |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0 |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0 |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0 |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0 |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0 |
| B10/r1/default | not_run; corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0 |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0 |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0 |
| Refused B13/r1 | `KN7:input.budget_not_declared`; rt4_library |
| Refused B14/r1 | `KN7:resource_limit.budget_ceiling`; rt4_library |
| Refused MR01/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR02/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR03/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR04/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR05/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR06/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR07/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR08/r2 | `PS1:capability.provenance_incompatible`; shared_policy |
| Refused MR09/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR10/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR11/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR12/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR15/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR16/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR17/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR18/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused P01/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P02/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P03/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P04/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused S01/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused S02/r1 | `LG5:input.module_digest_mismatch`; byte_approval |
| Decision / decider | open / owner |

### static_runtime_labels / source_bound

| Observation | Result |
| --- | --- |
| Declaration | not_run; RT4 attempt requires approved host subset and executed-byte binding; source_bound is not implemented |
| Must-refuse passes | 0/20; all 20 unrun, no enforcement evidence |
| Must-refuse failures | No executable result |
| Wrong registry codes | 0 recorded; unrun |
| Allowed tools | 0/35 allowed_ok |
| False refusals | 0/35; all allowed tools unrun |
| Planted reads | P01, P02, P03, P04 not_run; no containment evidence |
| Approval | S01 and S02 not_run |
| Reproducibility counterexamples | 0; 86 skipped identities with payloads; crash identity has no payload |
| Crashes / hangs | 10 / 0; B12/r1/shared in all ten repetitions, bench issue below |
| B01/r1/default | not_run; cell declaration |
| B02/r1/default | not_run; cell declaration |
| B03/r1/default | not_run; cell declaration |
| B04/r1/default | not_run; cell declaration |
| B05/r1/default | not_run; cell declaration |
| B06/r1/default | not_run; cell declaration |
| B07/r1/default | not_run; cell declaration |
| B08/r1/default | not_run; cell declaration |
| B09/r1/default | not_run; cell declaration |
| B10/r1/default | not_run; cell declaration |
| B11/r1/default | not_run; cell declaration |
| B12/r1/cold | not_run; cell declaration |
| B12/r1/warm | not_run; cell declaration |
| B12/r1/evicted | not_run; cell declaration |
| B12/r1/poisoned | not_run; cell declaration |
| B12/r1/shared | crash |
| B12/r1/relocated | not_run; cell declaration |
| B13/r1/default | not_run; cell declaration |
| B14/r1/default | not_run; cell declaration |
| Refused programs and codes | None; candidate did not execute |
| Decision / decider | open / owner |

### static_runtime_labels / approved_subset_byte_bound

| Observation | Result |
| --- | --- |
| Declaration | supported; experiment attempt |
| Must-refuse passes | 10/20 |
| Must-refuse failures | MR09, MR10, MR11, MR12, MR13, MR14, MR15, MR16, MR19, MR20 |
| Wrong registry codes | 0 |
| Allowed tools | 35/35 allowed_ok |
| False refusals | 0/35; none |
| Planted reads | 0; P01, P02, P03, P04 each refused_expected; zero trap calls |
| Approval | S01 and S02 refused_expected; zero dynamic executions and zero unapproved fragments |
| Reproducibility counterexamples | 0; 87 observation identities with exact payloads |
| Crashes / hangs | 0 / 0 |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0 |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0 |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0 |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6 |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0 |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0 |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0 |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0 |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0 |
| B10/r1/default | not_run; corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0 |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0 |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0 |
| Refused B13/r1 | `KN7:input.budget_not_declared`; rt4_library |
| Refused B14/r1 | `KN7:resource_limit.budget_ceiling`; rt4_library |
| Refused MR01/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR02/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR03/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR04/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR05/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR06/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR07/r2 | `LG3:capability.not_granted`; shared_policy |
| Refused MR08/r2 | `PS1:capability.provenance_incompatible`; shared_policy |
| Refused MR17/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused MR18/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused P01/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P02/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P03/r1 | `LG3:capability.ambient_read`; shared_policy |
| Refused P04/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused S01/r1 | `LG3:capability.not_granted`; shared_policy |
| Refused S02/r1 | `LG5:input.module_digest_mismatch`; byte_approval |
| Decision / decider | open / owner |

## Bench issues

1. **B12/shared with a declared `not_run` cell crashes in the bench.**
   `harness/matrix.py:65` returns before setting `shared_value_identity` at line
   90, but line 102 still requires that field from both concurrent results.
   Direct invocation of `worker` for the source-bound capability cell reproduces
   `KeyError: shared_value_identity`. The adapter never executes in that path.
   The affected keys remain crashes; the bench is unchanged. The integrator must
   resolve this in a separate bench change and regenerate pins/evidence if its
   bytes change.
2. **B10 stays unrun.** The frozen short loop does not establish the wall deadline,
   as `part-b/CORPUS-ISSUES.md` records. Supported cells return
   `not_run: corpus_issue`. Declared `not_run` source-bound cells return their cell
   reason before admission, so their B10 reason is that declaration, not the
   admission reason. Every B10 key remains present and unrun. No workaround or
   changed corpus is supplied, and no safety-stop evidence is claimed.

## Effort and limits

The candidate required a mechanical host-code renderer, approved-AST validation,
name and call-closure handling, abstract labels, branch/loop/module joins, byte
approval, wrapper values, domain-library wiring and reproducible artifact
production. The 137-line renderer, 228-line subset checker and 56-line adapter
show that adopting a host still duplicates substantial front-end work. The
232-line library implements candidate knowledge and operation wiring. Preparation
and acceptance add 292 lines, for 945 Python source lines excluding generated
programs. These are effort counts, not speed or maintainability measurements.

Reused: the frozen notation and fixtures and all shared policy, arithmetic,
metering, kernel, host, matrix and classification services. Throwaway: this
restricted Python spelling, four-slot fake-kernel integration, generated
programs and corpus-specific finite execution contract. No product module or
language design is offered for adoption.

The card cannot show **all-program safety, authorship, understanding, absence of
covert channels, product latency or an acceptable false-refusal threshold**.
This run also cannot select a host, prove real-engine correctness, establish an
OS sandbox or demonstrate DirectX, native input, long sessions or performance.
No speed is measured. Linux and the twenty-run cross-platform comparison are
**not run here**. Independent integration review remains with the integrator;
this one-folder, no-network packet excludes nested review-wrapper calls.

Read authorities, named rather than pinned to moving document bytes:
`AGENTS.md`, `docs/development-guide/AGENT_BRIEFING.md`, chapter 11's RTE.11,
RTE.12, RTE.24 to RTE.27, RTE.30 to RTE.37 and RTE.44 to RTE.46;
`docs/wiki/decisions/owner-decisions-2026-11-05.md`;
`work/experiments/language-lx/README.md`; the frozen corpus files;
`part-b/harness/README.md`, the named harness modules and both controls;
`part-b/RESULT.md` and `part-b/CORPUS-ISSUES.md`.

## Commands

Production from the repository root (writes candidate pins and results):

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt4-pyhost/prepare.py --runs 10
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt4-pyhost/prepare.py --seal
```

Acceptance from the repository root:

```text
python work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt4-pyhost/check.py
```

Acceptance verifies the raw corpus freeze and every frozen corpus file without
requiring historical chapter digests to match today's chapters. It verifies
candidate and artifact pins, regenerates all program bytes/source maps, runs one
Windows-labelled pass, compares keys, exact hashes, outcomes and small observer
evidence with the stored series, then deletes `build/` and checks every file hash.
A reproducible failed form or bench crash is a result; it does not become a pass
for that form. Acceptance never refreshes any stored artifact and cleans its
owned build output on failure too. The check's Windows label is the stored
replay contract; use the explicit Linux matrix command for Linux evidence.

For the still-unrun cross-platform comparison, the integrator must retain both
raw folders on one comparison host. Regenerate Windows raw results from the
repository root, since this packet deletes large raw files:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt4-pyhost --out candidates/rt4-pyhost/build/Windows --runs 10 --leg Windows
```

Linux leg, **not run here**, from the repository root on the Linux host:

```text
python3.14 -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt4-pyhost --out candidates/rt4-pyhost/build/Linux --runs 10 --leg headless_Linux
```

Combined report, **not run here**, from `work/experiments/language-lx/lx3-sandbox-flow/part-b/`:

```text
python -B -m harness.report --inputs candidates/rt4-pyhost/build/Windows candidates/rt4-pyhost/build/Linux --out candidates/rt4-pyhost/build/comparison.json
```

Keep raw outputs outside the retained patch and remove `build/` before rerunning
acceptance. No commit, stage, ref/config/hook change, network use, installation,
personal session or user data is part of this packet.
