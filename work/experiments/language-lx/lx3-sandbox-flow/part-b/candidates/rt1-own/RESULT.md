# E-2.4-L3 part B: RT1 own front end and interpreter

- LX card: **LX-3**, RTE.44 to RTE.46 and CT-RTE-05 in `docs/architecture/1.0/language/architecture/11-route-and-spikes.md`.
- Experiment ID: **E-2.4-L3 / part B / rt1-own**. Experiment code only; no product implementation.
- Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2, and this candidate's implementation packet.
- Time box: **what can be done now; one wrapper call of at most two hours**. A reached limit is no pass.
- Frozen candidate: `rt1-throwaway-prefix-v1`; raw bindings SHA-256 `c5b2aba702743b907c21a2cab772cafa88ddf0f5880063203e7d56a5e488847d`; source/dependency map digest `b9ccc6db273a46279b5d21f73256f2c538ea59b35c7eaf01bc7ac09eb00e8cd2`.
- Rendered program archive SHA-256: `59c9987232d14b0d0ab3fb47ae6c223fbcc88a5d8a20f28e0412f2c83d71c324`. `programs.json` indexes each program; `bindings.json` maps every neutral instruction, including unexecuted branches, to UTF-8 bytes.
- Frozen inputs: **lx3-neutral-v2**, raw `corpus/FREEZE.json` SHA-256 `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`; content digest `c4aa5b09430d8e5dbe02c63686b4fb400b36e7e7faf82dc42113491efe1e28c7`.
- `MANIFEST.json` pins every retained candidate file except itself. Acceptance snapshots the manifest too. Only experiment inputs/sources are pinned, never moving chapters or decision pages.
- Tools actually used: **CPython 3.15.1**, Python standard library only, Windows. No Node, native compiler, engine, installation, network or external service used.
- Evidence class: **source and synthetic**. The process-loss stimulus actually terminated a bench child on Windows; this is not product OS containment or Windows/DirectX evidence. No timing measurement or performance claim.
- Kill criteria: a wrong code, planted access, forbidden proposal/state change, unrefused prohibited stimulus, crash or hang fails its form/cell. Missing keys/pins or an exact reproducibility mismatch fail acceptance. Every harmless refusal is reported without a threshold. A later refusal cannot erase an earlier effect.
- Decision: **open**. Decider: **owner**. All LQs, route/engine/syntax choices, BS1 to BS4 and LP1 metering remain open.

## Actual result

The frozen Windows series ran **10 times per key**: **5220 recorded observations**, across all six declared cells. There are 87 observations per cell per repeat (82 neutral run keys, with B12's six cache conditions). The three source-bound cells execute this candidate. The three host-subset cells are declared not_run because RT1 has no host subset; their bench crash below remains a failure, never a pass. The Windows/Linux twenty-run requirement is **not complete**.

Capability separation has 20/20 expected-code must-refuse passes, and **two false refusals**, A25 and A30, both `LG3:capability.not_granted`. It rejects their combination of state reads and preview authority even though their previews use independent manual/fixed setup data. Both label forms have 16/20 must-refuse passes and **zero false refusals** among 35 tools. They leave **MR13, MR14, MR19 and MR20 unrefused**. These are the ordinary stop/emission and accumulated-observation gaps; this candidate supplies no unapproved repair. The failed forms remain failed.

All four planted accesses were refused with their expected codes and no trap call in each supported cell. All executable budget/cache/interruption keys matched their frozen outcomes. B10 remains `not_run: corpus_issue` in supported cells. S01/S02 are `not_run: variant_not_applicable` in source_bound under the frozen corpus. No case was removed or relabelled after observation. The separate mechanism probes exercise dynamic refusal and source/type/flow behavior; they do not change the frozen denominators or claim an S01/S02 matrix pass.

No supported-cell crashes/hangs or Windows reproducibility counterexamples occurred. In each unsupported cell B12/shared crashes in the bench, so 30 observations have a crash outcome and no exact payload/run key. All other 5190 observations retain hashes and expanded run keys. `summary.json` is the unmodified shared-bench summary, including every negative result and every not_run. `series.json` records raw output digests, byte sizes and their regeneration command; large JSONL was removed.

## Mechanism and limits

`render.py` accepts only a neutral program, never a case ID, family, expected result or prediction. `prepare.py` supplies program objects and separately builds the archive index. The surface is **throwaway**, a small prefix notation solely to give the checker real text. It is **no LQ13 syntax proposal**. Names and literal strings are escaped; the lexer tracks byte spans; the parser has a closed grammar and does not evaluate Python/JavaScript or deserialize neutral instruction objects. Input declarations contain types, while values, origins, author, profile and invocation authority remain frozen host inputs.

`rt1_check.py` validates literals, references, operation signatures, binding types, branch conditions, loop items and module arguments/results, and produces typed instructions over the parsed spans. Open Record/Rows projections have declared result types checked again before binding; this is not a fully structural record type system. It computes transitive effects, joins both branches, iterates loop and shared-call-site label summaries to a fixed point, and conservatively joins possible storage writes across sequential runs. Static labels are attached to the typed IR for exactly the bytes that execute.

`rt1_vm.py` owns immutable typed values, environments, module closures, calls, loops, return handling and branch context. Static mode uses the checker's labels at sinks and in produced blocks. Additional runtime-label mode computes actual value/control labels, checks them against the static bound, and submits those facts to `ctx.check`. Both arms are still type/effect checked. The independent probe with an unexecuted tainted arm demonstrates the more precise dynamic path; the frozen allowed list does not distinguish their refusal counts.

Calls join arguments, returned values and caller control. Modules close over immutable frozen input bindings, whose labels remain attached when captured values are read; there are no first-class/nested closure expressions in this corpus. Branch tests and loop collections enter the executor's control context, including constant-valued results under state-dependent branches. Aggregates carry the join of all operands/parts; field/index/alias operations retain source labels. Storage writes add data/control/profile provenance; reads consume the actual stored block and preserve it as a dependency. A new run never resets those labels. Research and unknown provenance survive copying and picking. Runtime storage labels must fit the inferred static summary.

Capability separation selects preview authority from the checked transitive effect closure; a program without preview loses that grant. The shared read grant also names fixed `caps`; shared policy excludes every state row when preview authority exists. The mechanism preflights those state-read effects through `ctx.check` before any proposal can be installed, and tracks stored read_state for later preview checks. This is conservative effect separation, including unexecuted branches and modules, not evidence that all inner flow computations ran in refused programs. Policy refusals are recorded by the actual `shared_policy` gate. Source mismatches use `rt1_source_binding`; budget admission uses `rt1_interpreter`/the shared meter.

Named-target setup uses the fixed shared table and records its host declassifier evidence. Pick records the actual manual event and all option/control dependencies; computed singletons keep their labels. Neither operator clears a tainted branch context. Policy decisions are exclusively `ctx.check`; the candidate does not read answer data. No stop-sensitive continuation repair, authorship proof, general language soundness proof or arbitrary covert-channel closure is claimed.

Every instruction and scalar constituent charges before its effect through `ctx.meter`. The pinned `harness.calc` primitive is reused for exact arithmetic and its specified visit/comparison order; it provides no interpreter control or labels. Kernel/cache/chunk service is `ctx.kernel_call`. Memory bindings, parameter/loop aliases, recursive 16-byte logical frames, output, capture and publication follow the shared order. Batch buffers are not charged twice. The meter is logical, not physical memory or elapsed time.

## Bench issues

1. **B12/shared in a not_run cell crashes before invoking this candidate.** Reproduced directly through `harness.matrix.worker`: `KeyError: shared_value_identity`. In `harness/matrix.py:65` the cell returns not_run before cache setup and before the flag at line 90 is attached; line 102 still indexes that flag for both shared workers. The frozen matrix SHA-256 is `2046930338b40a8aded4a2286a6992ea8edd8a9d8a7b3818801701f32f2642fa`. The candidate and acceptance preserve the three affected key classifications and do not patch the harness or manufacture exact records. The integrator must handle the bench defect separately; a changed harness changes this candidate's binding identity and needs a newly pinned series.
2. **B10** remains the integrator's documented corpus issue in `part-b/CORPUS-ISSUES.md`. No artificial delay/deadline was injected. In unsupported cells the bench's cell-level not_run check precedes B10 admission, so its recorded reason is the cell's no-host-subset reason; this still supplies no safety-stop evidence. No workaround or corpus change was made.

## capability_separation / source_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | supported |
| Must-refuse expected-code passes | 20/20; MR01, MR02, MR03, MR04, MR05, MR06, MR07, MR08, MR09, MR10, MR11, MR12, MR13, MR14, MR15, MR16, MR17, MR18, MR19, MR20 |
| Must-refuse failures | none observed |
| Wrong refusal codes | none observed |
| Allowed tools | 33/35 return the frozen value/observations |
| False refusals (denominator 35) | 2; A25: `LG3:capability.not_granted`; A30: `LG3:capability.not_granted` |
| Planted resources | P01: refused_expected; P02: refused_expected; P03: refused_expected; P04: refused_expected |
| Planted reads | 0 observed; no traps called in supported runs |
| Approval probes | S01: not_run; S02: not_run |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0; r1/body/2:before_step |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0; r1/body/1:before_kernel |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0; r1/body/1:before_memory |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6; r1/body/1:before_output |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0; r1/body/0:before_kernel |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0; r1/body/0:before_step |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0; r1/recurse:before_frame_4 |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0; r1/body/0:loop_continuation/1 |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0; r1/body/0:chunk/1 |
| B10/r1/default | not_run; corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0; r1/after_admission |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0; r1/admission |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0; r1/admission |
| Every refusal: `KN7:input.budget_not_declared` (rt1_interpreter) | B13/r1 |
| Every refusal: `KN7:resource_limit.budget_ceiling` (rt1_interpreter) | B14/r1 |
| Every refusal: `LG3:capability.ambient_read` (shared_policy) | P01/r1, P02/r1, P03/r1 |
| Every refusal: `LG3:capability.not_granted` (shared_policy) | A25/r1, A30/r1, MR01/r1, MR02/r1, MR03/r1, MR04/r2, MR05/r1, MR06/r1, MR07/r2, MR09/r1, MR10/r1, MR11/r1, MR12/r1, MR13/r1, MR14/r1, MR15/r1, MR16/r1, MR17/r1, MR18/r1, MR19/r1, MR19/r2, MR19/r3, MR20/r1, MR20/r2, MR20/r3, P04/r1 |
| Every refusal: `PS1:capability.provenance_incompatible` (shared_policy) | MR08/r2 |
| Reproducibility counterexamples | 0 in this Windows series; Linux not run here |
| Crashes / hangs | 0 crashes (10 repeats per listed key); 0 hangs |
| Recorded key outcomes | 87 per repeat, including every declared not_run and crash; none omitted |

Counters above are steps/kernel_cost/memory/output. Budget rows show repeat zero; all repeats and full coverage/stop records are in `summary.json`.

## capability_separation / approved_subset_byte_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | not_run: RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| Must-refuse expected-code passes | 0/20; none |
| Must-refuse failures | none observed |
| Wrong refusal codes | none observed |
| Allowed tools | 0/35 return the frozen value/observations |
| False refusals (denominator 35) | 0; none observed |
| Planted resources | P01: not_run; P02: not_run; P03: not_run; P04: not_run |
| Planted reads | 0 observed; this cell did not exercise a mechanism |
| Approval probes | S01: not_run; S02: not_run |
| B01/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B02/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B03/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B04/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B05/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B06/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B07/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B08/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B09/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B10/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B11/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/cold | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/warm | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/evicted | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/poisoned | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/shared | crash |
| B12/r1/relocated | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B13/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B14/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| Reproducibility counterexamples | 0 in this Windows series; Linux not run here |
| Crashes / hangs | B12/r1/shared crashes (10 repeats per listed key); 0 hangs |
| Recorded key outcomes | 87 per repeat, including every declared not_run and crash; none omitted |

Counters above are steps/kernel_cost/memory/output. Budget rows show repeat zero; all repeats and full coverage/stop records are in `summary.json`.

## static_labels / source_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | supported |
| Must-refuse expected-code passes | 16/20; MR01, MR02, MR03, MR04, MR05, MR06, MR07, MR08, MR09, MR10, MR11, MR12, MR15, MR16, MR17, MR18 |
| Must-refuse failures | MR13: unrefused; MR14: unrefused; MR19: unrefused; MR20: unrefused |
| Wrong refusal codes | none observed |
| Allowed tools | 35/35 return the frozen value/observations |
| False refusals (denominator 35) | 0; none observed |
| Planted resources | P01: refused_expected; P02: refused_expected; P03: refused_expected; P04: refused_expected |
| Planted reads | 0 observed; no traps called in supported runs |
| Approval probes | S01: not_run; S02: not_run |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0; r1/body/2:before_step |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0; r1/body/1:before_kernel |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0; r1/body/1:before_memory |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6; r1/body/1:before_output |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0; r1/body/0:before_kernel |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0; r1/body/0:before_step |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0; r1/recurse:before_frame_4 |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0; r1/body/0:loop_continuation/1 |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0; r1/body/0:chunk/1 |
| B10/r1/default | not_run; corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0; r1/after_admission |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0; r1/admission |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0; r1/admission |
| Every refusal: `KN7:input.budget_not_declared` (rt1_interpreter) | B13/r1 |
| Every refusal: `KN7:resource_limit.budget_ceiling` (rt1_interpreter) | B14/r1 |
| Every refusal: `LG3:capability.ambient_read` (shared_policy) | P01/r1, P02/r1, P03/r1 |
| Every refusal: `LG3:capability.not_granted` (shared_policy) | MR01/r1, MR02/r1, MR03/r1, MR04/r2, MR05/r1, MR06/r1, MR07/r2, MR09/r1, MR10/r1, MR11/r1, MR12/r1, MR15/r1, MR16/r1, MR17/r1, MR18/r1, P04/r1 |
| Every refusal: `PS1:capability.provenance_incompatible` (shared_policy) | MR08/r2 |
| Reproducibility counterexamples | 0 in this Windows series; Linux not run here |
| Crashes / hangs | 0 crashes (10 repeats per listed key); 0 hangs |
| Recorded key outcomes | 87 per repeat, including every declared not_run and crash; none omitted |

Counters above are steps/kernel_cost/memory/output. Budget rows show repeat zero; all repeats and full coverage/stop records are in `summary.json`.

## static_labels / approved_subset_byte_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | not_run: RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| Must-refuse expected-code passes | 0/20; none |
| Must-refuse failures | none observed |
| Wrong refusal codes | none observed |
| Allowed tools | 0/35 return the frozen value/observations |
| False refusals (denominator 35) | 0; none observed |
| Planted resources | P01: not_run; P02: not_run; P03: not_run; P04: not_run |
| Planted reads | 0 observed; this cell did not exercise a mechanism |
| Approval probes | S01: not_run; S02: not_run |
| B01/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B02/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B03/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B04/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B05/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B06/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B07/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B08/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B09/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B10/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B11/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/cold | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/warm | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/evicted | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/poisoned | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/shared | crash |
| B12/r1/relocated | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B13/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B14/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| Reproducibility counterexamples | 0 in this Windows series; Linux not run here |
| Crashes / hangs | B12/r1/shared crashes (10 repeats per listed key); 0 hangs |
| Recorded key outcomes | 87 per repeat, including every declared not_run and crash; none omitted |

Counters above are steps/kernel_cost/memory/output. Budget rows show repeat zero; all repeats and full coverage/stop records are in `summary.json`.

## static_runtime_labels / source_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | supported |
| Must-refuse expected-code passes | 16/20; MR01, MR02, MR03, MR04, MR05, MR06, MR07, MR08, MR09, MR10, MR11, MR12, MR15, MR16, MR17, MR18 |
| Must-refuse failures | MR13: unrefused; MR14: unrefused; MR19: unrefused; MR20: unrefused |
| Wrong refusal codes | none observed |
| Allowed tools | 35/35 return the frozen value/observations |
| False refusals (denominator 35) | 0; none observed |
| Planted resources | P01: refused_expected; P02: refused_expected; P03: refused_expected; P04: refused_expected |
| Planted reads | 0 observed; no traps called in supported runs |
| Approval probes | S01: not_run; S02: not_run |
| B01/r1/default | budget_ok; `KN7:budget.steps`; used 2/0/0/0; r1/body/2:before_step |
| B02/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 2/3/7/0; r1/body/1:before_kernel |
| B03/r1/default | budget_ok; `KN7:budget.memory`; used 2/0/16/0; r1/body/1:before_memory |
| B04/r1/default | budget_ok; `KN7:budget.output`; used 2/0/0/6; r1/body/1:before_output |
| B05/r1/default | budget_ok; `KN7:budget.kernel_cost`; used 1/0/0/0; r1/body/0:before_kernel |
| B06/r1/default | budget_ok; `KN7:budget.steps`; used 0/0/0/0; r1/body/0:before_step |
| B07/r1/default | budget_ok; `KN7:budget.memory`; used 4/0/48/0; r1/recurse:before_frame_4 |
| B08/r1/default | budget_ok; `KN7:cancelled.by_user`; used 3/0/1/0; r1/body/0:loop_continuation/1 |
| B09/r1/default | budget_ok; `KN7:cancelled.by_user`; used 1/2/117/0; r1/body/0:chunk/1 |
| B10/r1/default | not_run; corpus_issue |
| B11/r1/default | budget_ok; `KN7:resource_limit.runtime_lost`; used 0/0/0/0; r1/after_admission |
| B12/r1/cold | budget_ok; used 3/6/14/7 |
| B12/r1/warm | budget_ok; used 3/6/14/7 |
| B12/r1/evicted | budget_ok; used 3/6/14/7 |
| B12/r1/poisoned | budget_ok; used 3/6/14/7 |
| B12/r1/shared | budget_ok; used 3/6/14/7 |
| B12/r1/relocated | budget_ok; used 3/6/14/7 |
| B13/r1/default | budget_ok; `KN7:input.budget_not_declared`; used 0/0/0/0; r1/admission |
| B14/r1/default | budget_ok; `KN7:resource_limit.budget_ceiling`; used 0/0/0/0; r1/admission |
| Every refusal: `KN7:input.budget_not_declared` (rt1_interpreter) | B13/r1 |
| Every refusal: `KN7:resource_limit.budget_ceiling` (rt1_interpreter) | B14/r1 |
| Every refusal: `LG3:capability.ambient_read` (shared_policy) | P01/r1, P02/r1, P03/r1 |
| Every refusal: `LG3:capability.not_granted` (shared_policy) | MR01/r1, MR02/r1, MR03/r1, MR04/r2, MR05/r1, MR06/r1, MR07/r2, MR09/r1, MR10/r1, MR11/r1, MR12/r1, MR15/r1, MR16/r1, MR17/r1, MR18/r1, P04/r1 |
| Every refusal: `PS1:capability.provenance_incompatible` (shared_policy) | MR08/r2 |
| Reproducibility counterexamples | 0 in this Windows series; Linux not run here |
| Crashes / hangs | 0 crashes (10 repeats per listed key); 0 hangs |
| Recorded key outcomes | 87 per repeat, including every declared not_run and crash; none omitted |

Counters above are steps/kernel_cost/memory/output. Budget rows show repeat zero; all repeats and full coverage/stop records are in `summary.json`.

## static_runtime_labels / approved_subset_byte_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | not_run: RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| Must-refuse expected-code passes | 0/20; none |
| Must-refuse failures | none observed |
| Wrong refusal codes | none observed |
| Allowed tools | 0/35 return the frozen value/observations |
| False refusals (denominator 35) | 0; none observed |
| Planted resources | P01: not_run; P02: not_run; P03: not_run; P04: not_run |
| Planted reads | 0 observed; this cell did not exercise a mechanism |
| Approval probes | S01: not_run; S02: not_run |
| B01/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B02/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B03/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B04/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B05/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B06/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B07/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B08/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B09/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B10/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B11/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/cold | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/warm | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/evicted | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/poisoned | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B12/r1/shared | crash |
| B12/r1/relocated | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B13/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| B14/r1/default | not_run; RT1 has no host subset; RTE.45 assigns this variant to RT2 and RT4 |
| Reproducibility counterexamples | 0 in this Windows series; Linux not run here |
| Crashes / hangs | B12/r1/shared crashes (10 repeats per listed key); 0 hangs |
| Recorded key outcomes | 87 per repeat, including every declared not_run and crash; none omitted |

Counters above are steps/kernel_cost/memory/output. Budget rows show repeat zero; all repeats and full coverage/stop records are in `summary.json`.

## Production and acceptance

Commands from repository root (preparation intentionally refreshes candidate artifacts; acceptance never does):

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/tests.py
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/prepare.py --runs 10
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/report.py
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/prepare.py --seal
python work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/check.py
```

The last command verifies the whole frozen corpus (including its normalized self hash), all candidate and shared dependency pins, regenerated text/index/source-map bytes, and the mechanism probes. It makes one full Windows-labelled bench pass, checks every recorded outcome and exact hash/run key against every stored repeat, and removes `build/` even on failure. It leaves the candidate folder byte-identical and returns 0 for reproducible negative form results, including the declared-cell bench crashes. It writes no report, timing or refreshed pin. The report is frozen before that acceptance invocation.

## Effort and remaining work

This packet built the throwaway renderer, lexer/parser, type/effect and label analysis, typed IR, interpreter values/control/closures, adapter, mechanism probes, production/pinning and reproducibility check. It reused the unchanged policy, arithmetic tariff, kernel fixture, immutable host/provenance ledger, cache/interruption services, classifier, matrix and summary formats. The surface and interpreter are throwaway experiment work; none selects a production language architecture. The final Python sources total 1553 lines, including probes, reporting and artifact lifecycle. No third-party dependency was added.

Read guidance: `AGENTS.md`, `docs/development-guide/AGENT_BRIEFING.md`, the owner allowance and experiment index; chapter 11 RTE.11/12, RTE.24 to RTE.27, RTE.30 to RTE.37 and RTE.44 to RTE.46; the complete frozen corpus; the bench interface and named shared modules and both control formats; `part-b/RESULT.md` and `part-b/CORPUS-ISSUES.md`. The independent integration review remains with the integrator; this packet's no-network, one-folder boundary excludes nested review calls.

**Not run here:** the headless Linux leg and the combined twenty-run comparison. Since raw Windows output is deliberately removed, regenerate it on the comparison host, then retain both folders together:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt1-own --out candidates/rt1-own/build/Windows --runs 10 --leg Windows
python3.14 -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt1-own --out candidates/rt1-own/build/Linux --runs 10 --leg headless_Linux
```

Then from `work/experiments/language-lx/lx3-sandbox-flow/part-b/`:

```text
python -B -m harness.report --inputs candidates/rt1-own/build/Windows candidates/rt1-own/build/Linux --out candidates/rt1-own/build/comparison.json
```

The card cannot show **all-program safety, authorship, understanding, absence of covert channels, product latency or an acceptable false-refusal threshold**. This run additionally cannot establish the Linux leg, a twenty-run cross-platform result, wall-clock safety stops, real-engine correctness, native input, DirectX, long-session behavior or a product OS sandbox. No route choice, syntax proposal, LQ answer or release sign-off is made. Decision and decider remain **open / owner**.
