# LX-3 part B: rt3-wasm

Experiment only. Every decision remains **open**, decider **owner**. No route, engine language, syntax, LQ or BS option is selected.

## CT-RTE-05 record

| Field | Record |
| --- | --- |
| LX card / experiment | LX-3, RTE.44 to RTE.46 / E-2.4-L3, part B, candidate rt3-wasm |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`; this isolated implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours. A reached limit is no pass. |
| Frozen candidate | raw SHA-256 of `bindings.json`: `3150c75020ec9ebb14ab57659f914a73789b243d16250292e1b41cffd8458a1a`; its source_files pins the full candidate and shared Python closure |
| Frozen inputs | lx3-neutral-v2; raw FREEZE.json SHA-256 `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`; content SHA-256 `c4aa5b09430d8e5dbe02c63686b4fb400b36e7e7faf82dc42113491efe1e28c7` |
| Tools | CPython 3.15.1; Node v24.21.0 built-in WebAssembly engine; Python standard library and Node built-ins only |
| Evidence class | source and synthetic; Windows is the matrix leg, not a Windows/DirectX or performance claim |
| Kill criteria | A mismatch, planted read, unrefused prohibited case, crash, hang or wrong code fails that form. Missing keys cannot pass. |
| Actual series | 10 Windows runs per key, 5220 observations; all six cells record outcomes for all 87 invocation/host keys per repetition |
| Actual result | All three source-bound forms FAIL the must-refuse criterion. Capability separation: 18/20 cases pass, 2 fail, 2/35 false refusals. Each label form: 16/20 pass, 4 fail, 0/35 false refusals. |
| Decision / decider | open / owner; results do not select LQ9 or LQ21 |

**WebAssembly under Node's built-in engine stands in for a compiled target because no standalone WebAssembly VM is available.** This is the hybrid RT3 route: an own front end lowers to Wasm. It is not evidence for a standalone VM, native machine-code target, or the RT2/RT4 host-subset routes.

Cannot show: all-program safety, authorship, understanding, absence of covert channels, product latency, or an acceptable false-refusal threshold. Speed is not measured. No real engine, personal session, GPU device, native input, long session or product process boundary ran.

Read for this packet: `AGENTS.md`, `docs/development-guide/AGENT_BRIEFING.md`, chapter 11 RTE.11/RTE.12/RTE.24 to RTE.27/RTE.30 to RTE.37/RTE.44 to RTE.46, the owner allowance above, `work/experiments/language-lx/README.md`, the part-B harness README and modules, both controls' formats, all named frozen corpus inputs, and `part-b/CORPUS-ISSUES.md`. Moving chapter/decision pages are named, not pinned by this check.

## Mechanism and the RT3 lowering boundary

`render.py:render(program)` is a pure mechanical translation to JSON arrays with positional operands. It accepts no case ID, family or expected answer. It preserves both branches, source instruction paths and sequential entry order. This small surface is throwaway analysis notation, not an LQ13 syntax proposal. `programs.rt3` retains its exact UTF-8 source; `bindings.json` maps every neutral instruction, including unexecuted branches, to its exact byte range. `modules.json` retains every compiled module as hexadecimal plus its literal pool and lowering counts. No generated binary remains on disk.

`frontend.py:Analysis` joins both branches, iterates loop-carried masks to a fixed point, carries module summaries and tracks possible workspace labels across entries. Static mode emits immediate masks; it has zero emitted i32.or label-join sites. The uniform ABI still carries constant label fields and provenance handles in this mode. Runtime mode retains the structural/static analysis and emits operand/pc joins for the executed path. Its label masks are not taken from expected results. Labels are state=1, computed=2, research=4, unknown=8; read_state=16 occupies the same i32 field.

The handwritten assembler emits genuine Wasm if/block/loop/br instructions and direct function calls. Values stay as immutable handles on the Python host; Wasm carries control flow, explicit label fields and an i64 cumulative step-fuel global. User integers, exact collection operations and fake kernel values stay on the host, so Node never rounds A09's integer above 2^53. There is no claim to have compiled those numerical primitives into Wasm.

The lowering hazard is an implicit dependency becoming an ordinary native branch: choosing literal words under a state-derived condition would lose state if the compiler retained only each literal's data labels. `Compiler.block` emits a pc-mask OR in runtime/capability mode (an immediate analyzed mask in static mode) and a control-provenance handle at each branch and loop. Callees receive both fields; static joins retain the unexecuted arm as well. Effect intrinsics receive those computed fields and the control dependencies. `probe.py` runs independent synthetic branch/phi/module/workspace cases: state must survive literal selection, a compiled callee and persistence; static masks include an unexecuted state arm while runtime masks retain path precision. These are extra probes, never extra corpus passes.

Separation selects its effective grants before admission: for a solving entry with a reachable state row (both branches and transitive calls), it removes preview. Every gated intrinsic asks `ctx.check`; the shared policy alone supplies codes and permissions. Imported persisted read_state/profile/unknown metadata survives later runs. Capability mode keeps observational labels too; it is not a minimal-cost implementation of separation. A source scan alone does not close stop/emission channels, and the failed cases below remain failed.

Compiler-emitted step and loop-test imports charge before execution; the final false loop test is charged. Exact calculation constituents invoke the compiler-emitted `charge` Wasm export, reenter the host meter, then perform their arithmetic. Kernel, memory, output, batch-chunk cancellation and cache validation use the shared precharged intrinsics. Logical aliases and recursive frames follow the frozen tariff. The final Wasm fuel field must equal ctx.meter.steps. Costs are logical fixture counts, not physical instructions or timing.

The Node bridge has exactly 18 function imports and no WASI, memory or table imports. It reads/writes only its fixed stdio JSON protocol. The Python adapter is trusted glue, not an OS sandbox. Whole surface bytes and compiled modules are regenerated and compared before execution; source instructions are also recorded through ctx.executed. The mechanism reads only ctx stimulus/services and its pinned files; it does not read answer corpora. The report/preparation/check drivers do read frozen cases for enumeration and comparison.

The frozen S01/S02 cases apply only to approved_subset_byte_bound and are `not_run: variant_not_applicable` in source-bound cells. Separate synthetic probes confirm dynamic-evaluation refusal and detection of substituted dependency bytes before execution. Those probes do not change the frozen matrix's applicability. B11 kills the actual Python matrix child at admission, before Node starts; it is not a Node-kill or production process-containment test.

## Results by declared cell

A must-refuse case passes only when every gated run has the expected code and preservation/no-leak checks pass; harmless preparation runs are not refusals. Mixed success/failure in a multi-run case remains a failed case. Every denial is listed with its actual layer. No false-refusal threshold is imposed.

### capability_separation / source_bound

| Measure | Outcome |
| --- | --- |
| Declaration | supported; measured form FAILS |
| Must-refuse | 18/20 passed; failed: MR14, MR20 |
| Wrong codes | 0 recorded |
| False refusals | 2/35 recorded; A25 `LG3:capability.not_granted`; A30 `LG3:capability.not_granted` |
| Planted reads | 0; P01-P04 refused with expected codes; no trap calls/data |
| Budgets | B01-B07 and B13-B14: budget_ok, exact counters and stops |
| Cache | B12: cold, warm, evicted, poisoned, shared, relocated all budget_ok; exact hashes equal; poisoned entry rejected |
| Interruptions | B08 loop cancellation and B09 chunk cancellation: budget_ok; B11 actual child loss: budget_ok |
| Wall-clock safety stop | B10: not_run: corpus_issue |
| Approval probes | S01/S02: not_run: variant_not_applicable |
| Reproducibility counterexamples | 0; Windows only |
| Crashes / hangs | 0 / 0 |

Actual refused invocations (all repetitions agreed):

| Program/run | Code | Refusing layer |
| --- | --- | --- |
| A25/r1 | `LG3:capability.not_granted` | shared_policy |
| A30/r1 | `LG3:capability.not_granted` | shared_policy |
| B13/r1 | `KN7:input.budget_not_declared` | rt3_wasm |
| B14/r1 | `KN7:resource_limit.budget_ceiling` | rt3_wasm |
| MR01/r1 | `LG3:capability.not_granted` | shared_policy |
| MR02/r1 | `LG3:capability.not_granted` | shared_policy |
| MR03/r1 | `LG3:capability.not_granted` | shared_policy |
| MR04/r2 | `LG3:capability.not_granted` | shared_policy |
| MR05/r1 | `LG3:capability.not_granted` | shared_policy |
| MR06/r1 | `LG3:capability.not_granted` | shared_policy |
| MR07/r2 | `LG3:capability.not_granted` | shared_policy |
| MR08/r2 | `PS1:capability.provenance_incompatible` | shared_policy |
| MR09/r1 | `LG3:capability.not_granted` | shared_policy |
| MR10/r1 | `LG3:capability.not_granted` | shared_policy |
| MR11/r1 | `LG3:capability.not_granted` | shared_policy |
| MR12/r1 | `LG3:capability.not_granted` | shared_policy |
| MR13/r1 | `LG3:capability.not_granted` | shared_policy |
| MR15/r1 | `LG3:capability.not_granted` | shared_policy |
| MR16/r1 | `LG3:capability.not_granted` | shared_policy |
| MR17/r1 | `LG3:capability.not_granted` | shared_policy |
| MR18/r1 | `LG3:capability.not_granted` | shared_policy |
| MR19/r1 | `LG3:capability.not_granted` | shared_policy |
| MR19/r2 | `LG3:capability.not_granted` | shared_policy |
| MR19/r3 | `LG3:capability.not_granted` | shared_policy |
| MR20/r2 | `LG3:capability.not_granted` | shared_policy |
| P01/r1 | `LG3:capability.ambient_read` | shared_policy |
| P02/r1 | `LG3:capability.ambient_read` | shared_policy |
| P03/r1 | `LG3:capability.ambient_read` | shared_policy |
| P04/r1 | `LG3:capability.not_granted` | shared_policy |

### capability_separation / approved_subset_byte_bound

| Measure | Outcome |
| --- | --- |
| Declaration | not_run: own_front_end_has_no_host_subset |
| Must-refuse | 0/20 passed; 20 untested, denominator retained |
| Wrong codes | 0 recorded; not measured |
| False refusals | 0/35 recorded; 35 untested, denominator retained |
| Planted reads | not run |
| Budgets | not run |
| Cache | B12/shared: crash in the bench; other variants not run |
| Interruptions | not run |
| Wall-clock safety stop | B10: cell not run; B10 also remains a corpus issue |
| Approval probes | not run: own front end has no host subset |
| Reproducibility counterexamples | 0 exact-payload counterexamples; B12/shared has no payload; other not_run envelopes repeat |
| Crashes / hangs | 10 / 0: one B12/shared key, ten repeats; see Bench issues |

### static_labels / source_bound

| Measure | Outcome |
| --- | --- |
| Declaration | supported; measured form FAILS |
| Must-refuse | 16/20 passed; failed: MR13, MR14, MR19, MR20 |
| Wrong codes | 0 recorded |
| False refusals | 0/35 recorded; none |
| Planted reads | 0; P01-P04 refused with expected codes; no trap calls/data |
| Budgets | B01-B07 and B13-B14: budget_ok, exact counters and stops |
| Cache | B12: cold, warm, evicted, poisoned, shared, relocated all budget_ok; exact hashes equal; poisoned entry rejected |
| Interruptions | B08 loop cancellation and B09 chunk cancellation: budget_ok; B11 actual child loss: budget_ok |
| Wall-clock safety stop | B10: not_run: corpus_issue |
| Approval probes | S01/S02: not_run: variant_not_applicable |
| Reproducibility counterexamples | 0; Windows only |
| Crashes / hangs | 0 / 0 |

Actual refused invocations (all repetitions agreed):

| Program/run | Code | Refusing layer |
| --- | --- | --- |
| B13/r1 | `KN7:input.budget_not_declared` | rt3_wasm |
| B14/r1 | `KN7:resource_limit.budget_ceiling` | rt3_wasm |
| MR01/r1 | `LG3:capability.not_granted` | shared_policy |
| MR02/r1 | `LG3:capability.not_granted` | shared_policy |
| MR03/r1 | `LG3:capability.not_granted` | shared_policy |
| MR04/r2 | `LG3:capability.not_granted` | shared_policy |
| MR05/r1 | `LG3:capability.not_granted` | shared_policy |
| MR06/r1 | `LG3:capability.not_granted` | shared_policy |
| MR07/r2 | `LG3:capability.not_granted` | shared_policy |
| MR08/r2 | `PS1:capability.provenance_incompatible` | shared_policy |
| MR09/r1 | `LG3:capability.not_granted` | shared_policy |
| MR10/r1 | `LG3:capability.not_granted` | shared_policy |
| MR11/r1 | `LG3:capability.not_granted` | shared_policy |
| MR12/r1 | `LG3:capability.not_granted` | shared_policy |
| MR15/r1 | `LG3:capability.not_granted` | shared_policy |
| MR16/r1 | `LG3:capability.not_granted` | shared_policy |
| MR17/r1 | `LG3:capability.not_granted` | shared_policy |
| MR18/r1 | `LG3:capability.not_granted` | shared_policy |
| P01/r1 | `LG3:capability.ambient_read` | shared_policy |
| P02/r1 | `LG3:capability.ambient_read` | shared_policy |
| P03/r1 | `LG3:capability.ambient_read` | shared_policy |
| P04/r1 | `LG3:capability.not_granted` | shared_policy |

### static_labels / approved_subset_byte_bound

| Measure | Outcome |
| --- | --- |
| Declaration | not_run: own_front_end_has_no_host_subset |
| Must-refuse | 0/20 passed; 20 untested, denominator retained |
| Wrong codes | 0 recorded; not measured |
| False refusals | 0/35 recorded; 35 untested, denominator retained |
| Planted reads | not run |
| Budgets | not run |
| Cache | B12/shared: crash in the bench; other variants not run |
| Interruptions | not run |
| Wall-clock safety stop | B10: cell not run; B10 also remains a corpus issue |
| Approval probes | not run: own front end has no host subset |
| Reproducibility counterexamples | 0 exact-payload counterexamples; B12/shared has no payload; other not_run envelopes repeat |
| Crashes / hangs | 10 / 0: one B12/shared key, ten repeats; see Bench issues |

### static_runtime_labels / source_bound

| Measure | Outcome |
| --- | --- |
| Declaration | supported; measured form FAILS |
| Must-refuse | 16/20 passed; failed: MR13, MR14, MR19, MR20 |
| Wrong codes | 0 recorded |
| False refusals | 0/35 recorded; none |
| Planted reads | 0; P01-P04 refused with expected codes; no trap calls/data |
| Budgets | B01-B07 and B13-B14: budget_ok, exact counters and stops |
| Cache | B12: cold, warm, evicted, poisoned, shared, relocated all budget_ok; exact hashes equal; poisoned entry rejected |
| Interruptions | B08 loop cancellation and B09 chunk cancellation: budget_ok; B11 actual child loss: budget_ok |
| Wall-clock safety stop | B10: not_run: corpus_issue |
| Approval probes | S01/S02: not_run: variant_not_applicable |
| Reproducibility counterexamples | 0; Windows only |
| Crashes / hangs | 0 / 0 |

Actual refused invocations (all repetitions agreed):

| Program/run | Code | Refusing layer |
| --- | --- | --- |
| B13/r1 | `KN7:input.budget_not_declared` | rt3_wasm |
| B14/r1 | `KN7:resource_limit.budget_ceiling` | rt3_wasm |
| MR01/r1 | `LG3:capability.not_granted` | shared_policy |
| MR02/r1 | `LG3:capability.not_granted` | shared_policy |
| MR03/r1 | `LG3:capability.not_granted` | shared_policy |
| MR04/r2 | `LG3:capability.not_granted` | shared_policy |
| MR05/r1 | `LG3:capability.not_granted` | shared_policy |
| MR06/r1 | `LG3:capability.not_granted` | shared_policy |
| MR07/r2 | `LG3:capability.not_granted` | shared_policy |
| MR08/r2 | `PS1:capability.provenance_incompatible` | shared_policy |
| MR09/r1 | `LG3:capability.not_granted` | shared_policy |
| MR10/r1 | `LG3:capability.not_granted` | shared_policy |
| MR11/r1 | `LG3:capability.not_granted` | shared_policy |
| MR12/r1 | `LG3:capability.not_granted` | shared_policy |
| MR15/r1 | `LG3:capability.not_granted` | shared_policy |
| MR16/r1 | `LG3:capability.not_granted` | shared_policy |
| MR17/r1 | `LG3:capability.not_granted` | shared_policy |
| MR18/r1 | `LG3:capability.not_granted` | shared_policy |
| P01/r1 | `LG3:capability.ambient_read` | shared_policy |
| P02/r1 | `LG3:capability.ambient_read` | shared_policy |
| P03/r1 | `LG3:capability.ambient_read` | shared_policy |
| P04/r1 | `LG3:capability.not_granted` | shared_policy |

### static_runtime_labels / approved_subset_byte_bound

| Measure | Outcome |
| --- | --- |
| Declaration | not_run: own_front_end_has_no_host_subset |
| Must-refuse | 0/20 passed; 20 untested, denominator retained |
| Wrong codes | 0 recorded; not measured |
| False refusals | 0/35 recorded; 35 untested, denominator retained |
| Planted reads | not run |
| Budgets | not run |
| Cache | B12/shared: crash in the bench; other variants not run |
| Interruptions | not run |
| Wall-clock safety stop | B10: cell not run; B10 also remains a corpus issue |
| Approval probes | not run: own front end has no host subset |
| Reproducibility counterexamples | 0 exact-payload counterexamples; B12/shared has no payload; other not_run envelopes repeat |
| Crashes / hangs | 10 / 0: one B12/shared key, ten repeats; see Bench issues |

Stop/emission counterexamples are MR13/MR14; accumulation counterexamples are MR19/MR20. Both label forms miss all four. Separation misses MR14 and MR20 because some paths stop before attempting the removed preview capability; other MR20 runs do refuse, which cannot rescue the case. Labels do not claim a termination-sensitive repair. A25 and A30 are the separation false refusals: they display state-derived facts and then request a manual/fixed-setup preview; the selected grant subset denies that preview.

`summary.json` is the shared bench's unedited summary. `outcomes.json` retains one classification, run key and exact payload hash for every invocation/host key; repeat disagreement prevents capture. Full payloads are regenerated, not retained: `evidence.json` records their digest and size and pins the small retained tables and report. No absolute paths or raw machine diagnostics are retained.

## Lowering counts

Per neutral program, summed across its separately compiled entries: added i32 label locals/parameters + i64 fuel fields / total emitted call sites. These are static counts, not executed-call frequencies, latency or benchmark measurements. Result-label signature fields, control fields, temporary masks, i32.or sites, per-import call counts and module byte counts are individually retained under each program/form/entry in `modules.json`. Full source digests are the keys there. No timing comparison is made.

| Neutral program SHA-256 prefix | Entries | Separation | Static | Runtime |
| --- | --- | --- | --- | --- |
| `03552965abb78857` | 3 | 72 + 3 / 117 | 72 + 3 / 111 | 72 + 3 / 117 |
| `03db047663c4881c` | 1 | 15 + 1 / 23 | 15 + 1 / 21 | 15 + 1 / 23 |
| `0ba146444444e6f0` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `0c3a4a1854de174a` | 1 | 12 + 1 / 18 | 12 + 1 / 16 | 12 + 1 / 18 |
| `0d6ee5dffc9e9a6c` | 1 | 8 + 1 / 12 | 8 + 1 / 12 | 8 + 1 / 12 |
| `0f8167b24fdf3374` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `17597baec26d94e5` | 1 | 14 + 1 / 22 | 14 + 1 / 20 | 14 + 1 / 22 |
| `1f2c6cf5f33d76cb` | 1 | 11 + 1 / 16 | 11 + 1 / 14 | 11 + 1 / 16 |
| `21dd8c85b4248ff3` | 1 | 21 + 1 / 32 | 21 + 1 / 28 | 21 + 1 / 32 |
| `2583293bdfd164e7` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `267eb14b4423ff2c` | 1 | 7 + 1 / 7 | 7 + 1 / 7 | 7 + 1 / 7 |
| `2f0b8c7f2c25beac` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `39b70758ddfc2627` | 1 | 8 + 1 / 17 | 8 + 1 / 17 | 8 + 1 / 17 |
| `3ff25e01b9ad2a58` | 1 | 13 + 1 / 21 | 13 + 1 / 19 | 13 + 1 / 21 |
| `41bfec18a881abe5` | 1 | 10 + 1 / 17 | 10 + 1 / 15 | 10 + 1 / 17 |
| `4a6fb99c23ff27fc` | 2 | 15 + 2 / 25 | 15 + 2 / 20 | 15 + 2 / 25 |
| `4d70859964dd9c17` | 1 | 3 + 1 / 4 | 3 + 1 / 4 | 3 + 1 / 4 |
| `53fa9562cc7cf8a9` | 1 | 8 + 1 / 12 | 8 + 1 / 10 | 8 + 1 / 12 |
| `5ad9c95638661e1f` | 1 | 24 + 1 / 36 | 24 + 1 / 31 | 24 + 1 / 36 |
| `5ec1826819ac88ae` | 1 | 7 + 1 / 10 | 7 + 1 / 10 | 7 + 1 / 10 |
| `7259a1725a7015e9` | 1 | 29 + 1 / 51 | 29 + 1 / 47 | 29 + 1 / 51 |
| `75d5e15060453c0f` | 1 | 8 + 1 / 12 | 8 + 1 / 10 | 8 + 1 / 12 |
| `770379f2332fb7d6` | 1 | 6 + 1 / 10 | 6 + 1 / 8 | 6 + 1 / 10 |
| `78e52a8ccd9cd9bd` | 1 | 9 + 1 / 16 | 9 + 1 / 16 | 9 + 1 / 16 |
| `7b653ef52b642f9a` | 3 | 63 + 3 / 99 | 63 + 3 / 93 | 63 + 3 / 99 |
| `7bbefe1cc831692b` | 1 | 8 + 1 / 12 | 8 + 1 / 10 | 8 + 1 / 12 |
| `7df859b7c67a578c` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `800597b7e8900ef3` | 1 | 25 + 1 / 39 | 25 + 1 / 37 | 25 + 1 / 39 |
| `86b4735556a8d70f` | 1 | 4 + 1 / 4 | 4 + 1 / 4 | 4 + 1 / 4 |
| `8a5e786ae76e97a4` | 1 | 19 + 1 / 29 | 19 + 1 / 28 | 19 + 1 / 29 |
| `8c8a1070802324a0` | 1 | 7 + 1 / 11 | 7 + 1 / 11 | 7 + 1 / 11 |
| `8eed0c4cf9ef6114` | 1 | 19 + 1 / 27 | 19 + 1 / 25 | 19 + 1 / 27 |
| `9062d00eef6d3c4d` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `9696dbed7e94713e` | 1 | 20 + 1 / 39 | 20 + 1 / 35 | 20 + 1 / 39 |
| `972fe5669645c940` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `9f7a5318da3a2472` | 1 | 8 + 1 / 12 | 8 + 1 / 12 | 8 + 1 / 12 |
| `9fdabf5c147d6daa` | 1 | 8 + 1 / 12 | 8 + 1 / 12 | 8 + 1 / 12 |
| `a6f24dd5be5766ec` | 1 | 29 + 1 / 49 | 29 + 1 / 45 | 29 + 1 / 49 |
| `a730aa1c6bf1955b` | 1 | 7 + 1 / 11 | 7 + 1 / 11 | 7 + 1 / 11 |
| `ac7b32209bbcf06c` | 1 | 15 + 1 / 23 | 15 + 1 / 21 | 15 + 1 / 23 |
| `ad96ebbe9ba837d1` | 1 | 17 + 1 / 26 | 17 + 1 / 22 | 17 + 1 / 26 |
| `b24de696e4c9cc09` | 1 | 6 + 1 / 13 | 6 + 1 / 13 | 6 + 1 / 13 |
| `b29967a88c8970ff` | 1 | 16 + 1 / 24 | 16 + 1 / 22 | 16 + 1 / 24 |
| `b2c68c844564eda2` | 1 | 15 + 1 / 23 | 15 + 1 / 21 | 15 + 1 / 23 |
| `b658f3abf1190d36` | 1 | 25 + 1 / 43 | 25 + 1 / 41 | 25 + 1 / 43 |
| `b8cfd0020b3f90c5` | 1 | 14 + 1 / 22 | 14 + 1 / 19 | 14 + 1 / 22 |
| `b93756a70c3913cb` | 1 | 8 + 1 / 12 | 8 + 1 / 12 | 8 + 1 / 12 |
| `ba339ad3f3f17b4f` | 1 | 37 + 1 / 65 | 37 + 1 / 59 | 37 + 1 / 65 |
| `bb7bd75d0209a3d1` | 1 | 8 + 1 / 12 | 8 + 1 / 10 | 8 + 1 / 12 |
| `bc49cc08f4e36a5a` | 1 | 12 + 1 / 18 | 12 + 1 / 16 | 12 + 1 / 18 |
| `bd639cc3ea565ba8` | 1 | 7 + 1 / 7 | 7 + 1 / 7 | 7 + 1 / 7 |
| `bea9bdf73d5b0f74` | 1 | 18 + 1 / 27 | 18 + 1 / 25 | 18 + 1 / 27 |
| `c08008cad7b0c945` | 1 | 9 + 1 / 15 | 9 + 1 / 13 | 9 + 1 / 15 |
| `c1102ee7e6cf889f` | 1 | 9 + 1 / 11 | 9 + 1 / 11 | 9 + 1 / 11 |
| `c55e5a18eb6e9cc7` | 1 | 5 + 1 / 6 | 5 + 1 / 6 | 5 + 1 / 6 |
| `c937dcae4de931e8` | 1 | 20 + 1 / 30 | 20 + 1 / 28 | 20 + 1 / 30 |
| `c9acac2c21aee9aa` | 1 | 8 + 1 / 13 | 8 + 1 / 11 | 8 + 1 / 13 |
| `cebaa07a68efde27` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `cf741ec0640e21a1` | 1 | 7 + 1 / 11 | 7 + 1 / 10 | 7 + 1 / 11 |
| `d66d262c7f16d951` | 1 | 8 + 1 / 14 | 8 + 1 / 12 | 8 + 1 / 14 |
| `d78bda5d8e425283` | 1 | 18 + 1 / 27 | 18 + 1 / 25 | 18 + 1 / 27 |
| `da286d48a1b18be4` | 1 | 11 + 1 / 18 | 11 + 1 / 16 | 11 + 1 / 18 |
| `da67377811f5f14f` | 2 | 33 + 2 / 53 | 33 + 2 / 44 | 33 + 2 / 53 |
| `db7f03ea6d46113c` | 1 | 17 + 1 / 26 | 17 + 1 / 24 | 17 + 1 / 26 |
| `e84a7f6d8a1c64be` | 1 | 25 + 1 / 43 | 25 + 1 / 41 | 25 + 1 / 43 |
| `e98f73abc3870fd7` | 2 | 39 + 2 / 69 | 39 + 2 / 60 | 39 + 2 / 69 |
| `f24d749c83564129` | 1 | 22 + 1 / 36 | 22 + 1 / 34 | 22 + 1 / 36 |
| `f24db3420fce7d35` | 1 | 20 + 1 / 32 | 20 + 1 / 30 | 20 + 1 / 32 |
| `f5fe34960995ebe6` | 1 | 17 + 1 / 26 | 17 + 1 / 24 | 17 + 1 / 26 |
| `f671a196928ce98b` | 1 | 24 + 1 / 44 | 24 + 1 / 40 | 24 + 1 / 44 |
| `f8def1342737891f` | 1 | 10 + 1 / 16 | 10 + 1 / 14 | 10 + 1 / 16 |

## Commands and reproducibility

From the repository root; CLI paths after matrix flags are relative to part-b. Preparation intentionally writes pins and is not an acceptance operation:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt3-wasm/prepare.py
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt3-wasm/probe.py
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt3-wasm --out candidates/rt3-wasm/build/Windows --runs 10 --leg Windows
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt3-wasm/report.py build/Windows
python work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt3-wasm/check.py
```

The report command captures the finished series and writes the initial tables. The final applicability corrections and Bench issues addendum are hand-authored in this editable Markdown and pinned by evidence.json; the capture helper does not regenerate that addendum. Remove the candidate-owned build folder after capture. Acceptance verifies the complete corpus freeze without checking moving source-document digests, verifies source/binding pins, regenerates both rendered files byte for byte, runs the focused probes and one full Windows matrix pass, compares every outcome/run key/exact hash, and removes build even on failure. Acceptance does not refresh any pin, result, report or timing field. A negative form can reproduce successfully; acceptance success is not a form pass.

**Not run here: headless Linux and the combined twenty-run series.** On the comparison host, regenerate both legs (raw outputs are not shipped) with exactly:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt3-wasm --out candidates/rt3-wasm/build/Windows --runs 10 --leg Windows
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt3-wasm --out candidates/rt3-wasm/build/Linux --runs 10 --leg headless_Linux
```

Then from `work/experiments/language-lx/lx3-sandbox-flow/part-b/`:

```text
python -B -m harness.report --inputs candidates/rt3-wasm/build/Windows candidates/rt3-wasm/build/Linux --out candidates/rt3-wasm/build/comparison.json
```

Record the Linux Python/Node versions before comparison. Do not refresh bindings to relabel a changed candidate. No GPU or decisive timing command is required for this card; neither was run here. Integrator review remains outside this implementation packet; no commit, stage, ref change, nested review call, network access or installation was performed.

## Bench issues

B10 remains `not_run: corpus_issue` at admission, exactly as the integrator's recorded decision requires. Its short loop does not sustain the wall deadline; no delay or synthetic trigger was inserted. In wholly not-run host-subset cells the shared bench reports the cell reason first; B10 is still untested, never passed. This is the documented bench behavior, not a denominator exclusion. No bench or corpus file was changed, and no other defect was worked around.



B12 has a separate **shared-cache worker defect for not_run cells**. Each of the three `approved_subset_byte_bound` cells records `crash` for `B12/r1/shared` in every repetition: 30 crash observations, three distinct keys. The adapter is never entered for these requests. `harness/matrix.py:65` returns the not_run record before line 90 adds `shared_value_identity`; line 102 then unconditionally reads that absent field and raises `KeyError: shared_value_identity`. Matching source SHA-256: `2046930338b40a8aded4a2286a6992ea8edd8a9d8a7b3818801701f32f2642fa`. A direct worker invocation reproduced this error. No workaround, declaration change, or bench edit was made.

Reproduction from `work/experiments/language-lx/lx3-sandbox-flow/part-b/` (expected to fail with that KeyError):

```text
python -B -c "from harness.matrix import worker; worker(dict(candidate='candidates/rt3-wasm', case_id='B12', form='capability_separation', variant='approved_subset_byte_bound', host_variant='shared'))"
```

The outcome ledger retains those three crashes with null run key and payload hash; the shared summary has no exact payload for them. They are failed observations, not missing matrix rows or exact-reproducibility successes. The other 519 distinct keys have exact hashes, with no counterexamples across ten Windows repetitions. In the supported source-bound cells, all six cache variants ran successfully and no crash or hang occurred. An integrator fix to the pinned bench would require a new identified measurement; this candidate keeps the defect visible.

## Effort and unfinished work

Built one positional-array renderer, a finite front-end analysis and handwritten Wasm binary assembler, explicit handle/label/control ABI, synchronous stdio bridge with a reentrant metering trampoline, trusted host-operation glue, source/compiled-byte pins, synthetic lowering probes, and capture/acceptance drivers. Reused the frozen corpus and the shared policy, exact calculations, fake kernel, immutable provenance host, meter, cache checks, classifier, matrix and summary; no enforcing mechanism was borrowed from a control. The surface, assembler, bridge and fixture-specific plumbing are throwaway experiment code. There is no proposed product language or general-purpose compiler.

Open: the B12 not_run/shared worker defect; all form failures and the false-refusal tradeoff; all owner decisions; Linux comparison; B10 wall-clock evidence; standalone Wasm runtime evidence; all-program correctness/security and product integration. The three approved_subset_byte_bound cells deliberately remain not_run because this route has no host subset. These omissions and failures do not become passes at the time limit.
