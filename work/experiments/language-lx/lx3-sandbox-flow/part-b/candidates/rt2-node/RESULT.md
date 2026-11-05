# LX-3 part B: rt2-node

This is experiment code under `work/experiments/`, never product code. It selects
no route, engine language or syntax and answers no LQ. Every decision is **open**;
the decider is the **owner**. A failed form is an experiment result.

## Card and frozen identity (CT-RTE-05)

- LX card: **LX-3**, RTE.44 to RTE.46, chapter 11, CT-RTE-05.
- Experiment ID: **E-2.4-L3, part B, rt2-node**, bench interface v1.
- Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2,
  and this candidate's assigned implementation packet.
- Time box: **what can be done now; one wrapper call of at most two hours**.
  A reached limit is no pass.
- Frozen input: **lx3-neutral-v2**. Raw
  `work/experiments/language-lx/lx3-sandbox-flow/corpus/FREEZE.json` SHA-256:
  `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`.
  Its content SHA-256 is
  `c4aa5b09430d8e5dbe02c63686b4fb400b36e7e7faf82dc42113491efe1e28c7`.
- Frozen candidate: `bindings.json` pins the executed candidate Python/JavaScript
  source, rendered slices/index, shared harness modules and runner. `tools.json`
  pins executable hashes and the reported Node dependency versions. `MANIFEST.json`
  pins the retained candidate files, including report, summaries and check.
- Tools run here: **CPython 3.15.1**, **Node 24.21.0**,
  **V8 13.6.233.17-node.53**, Windows 11; Python standard library and Node built-ins.
  No npm, pip, installation, network, real session or third-party package was used.
- Evidence class: **source and synthetic**. The Windows child-process tests below
  are synthetic observations, not Windows/DirectX, engine or performance evidence.
- Kill criteria: missing outcomes, changed source/input/tool pins, differing
  run keys or exact payloads fail reproducibility. Successful planted reads,
  unrefused prohibited cases, wrong codes, forbidden state changes, crashes or
  hangs fail the relevant form. False refusals have no threshold. Neither a late
  refusal nor a failed limit erases an earlier effect.
- Decisions: **open**, decider **owner**: LQ2, LQ9, LQ13, LQ21, BS1 to BS4 and
  LP1 metering. No architecture decision or ADR is accepted.

## Mechanism and scope of the checker

`render.py` takes only a neutral program. It emits native JavaScript function
calls, branches and loops, with all neutral instructions mapped to UTF-8 byte
spans, including unexecuted arms and modules. It never receives a case ID, family,
expected code or outcome. `programs.js` concatenates independent program slices;
`program-index.json` selects one complete slice for each stimulus. The adapter
renders the current `ctx.program`, including a substituted dependency, and checks
it against the originally pinned slice before execution. The candidate never
silently substitutes the approved dependency for the changed one.

The hand-written subset recognizer in `checker.py` reads the JSON header from the
actual JavaScript bytes, validates the closed instruction vocabulary and
reconstructs every permitted JavaScript token. Exact byte equality is mandatory.
Only the renderer's canonical image is accepted: no arbitrary JS expressions,
property walks, comments, whitespace edits, imports, eval or added statements.
This is a deliberately narrow byte/template checker, not a general JavaScript
parser or a proposed language syntax. The pinned corpus validates neutral types;
this experiment adds no general host-language type checker. Dynamic and ambient
authority attempts are identified before Node starts; `ctx.check` supplies their
refusal codes. All other effects ask the same policy before performing an effect.
No mechanism code reads answer files or expected outcomes.

The candidate computes static may-labels itself. It joins both branch arms,
uses a finite fixed point for loop-carried bindings, follows module calls and
retains abstract workspace dependencies across sequential host runs. Recursive
cycles conservatively use all labels. Data and control dependencies go to the
host-built provenance blocks; programs cannot supply those blocks. Static labels
do not become executed-path labels. Stop and return do not taint subsequent
statements, so no stop-sensitive repair has been added.

For capability separation, transitive preview presence fixes the grant subset
before admission. Read-only solving programs lose the preview grant. In a
previewing solving program, the shared policy prevents state-row reads; stored
read-state and profile provenance also constrain later previews. A program-wide
read flag is conservative and does not instrument VM values. Some permitted
manual previews can occur before a later read is refused (MR13 and MR19);
the host retains those observations and does not invent rollback. Refusal
checkpoint preservation and the frozen classification rule determine the count.

The Python adapter starts one Node child for a worker that reaches execution and
reuses it for sequential runs. Each run has a new `vm` context. Granted functions
carry JSON requests and replies over stdio to Python; no process, file, network,
clock or dynamic-evaluation API is granted. Exact values and host provenance stay
in Python behind opaque handles. JavaScript runs control flow, while exact
arithmetic and constituent charges reuse `harness/calc.py`. In particular, A09's
integer beyond 2^53 never becomes a JavaScript Number. This is not evidence about
native JavaScript big-integer performance or an independent arithmetic kernel.

Instruction, loop-test, recursive-frame, kernel, binding and output charges occur
through `ctx.meter` before the corresponding effect. The host preserves kernel
cache validation, ordered batch cancellation, receipts, picks and provenance.
B12's two overlapping Python contexts share an immutable cached value, while a
lock serializes their requests to the one Node process. This is not simultaneous
execution of two JavaScript VMs or a cache-speed measurement.

Run-time labels are **not_run**: RTE.12 and RTE.24 require modification of the
borrowed VM; this Node/V8 is unmodified. The source-bound variant is **not_run**:
the implemented RT2 variant is `approved_subset_byte_bound`.

## Limits and interruption

All ordinary continuations are charged in Python. A non-cooperative JavaScript
loop with no host call is interrupted by `vm.Script`'s 2,000 ms timeout and mapped
to typed `KN7:resource_limit.safety_stop`. The adapter also has a 6-second
inter-message watchdog that kills and reaps Node; this fallback is implemented
but was not independently forced. The shared matrix has its own 10-second child
watchdog. These wall limits are containment leads, never deterministic budgets.

The local probe actually ran an unchecked no-host-call loop through the adapter
and observed that typed safety stop. It then killed Node and observed typed
`KN7:resource_limit.runtime_lost`. Parent-pipe EOF ends Node, and normal stop
messages unwind an invocation without granting the program exception handling.
No native host callback hang, host exhaustion, product process or durable store
was tested.

Corpus B08 and B09 exercise the frozen cooperative loop and batch interrupts.
B11 kills the bench's Python child at admission, before Node is started; its
observed result must not be relabelled as a Node kill. The separate local Node
kill supplies that narrower observation. B10 supplies no wall-clock-stop evidence.

## RTE.11 questions and local source facts

| Question | Observation and limit |
| --- | --- |
| Which host facilities can the approved subset reach? | Only its rendered bridge calls and compiler-owned control scaffolding. Effects remain behind `ctx.check`. The unchecked VM has native intrinsics and host-callback escape paths, as the separate table records. |
| Can dynamic code be refused and approval bind executed bytes? | S01 is refused before evaluation; S02 changes the transitive module bytes and is refused before execution. Acceptance regenerates all bytes and source spans; the host records each executed fragment against its approved digest. This covers this canonical subset and frozen dependency closure. |
| Are limits and interrupts enforceable? | Logical budgets, cooperative interruptions and local timeout/kill observations are listed below. B10, physical exhaustion and blocking native host callbacks remain unproved. |
| Do exact results repeat? | The retained Windows series and fresh acceptance pass compare full run keys and exact payload hashes, including stops, coverage and counters. No Linux result is claimed. |
| Dependency and modification licence obligations? | **not known here**. No local Node/V8 licence or documentation file was established, and none is cited for a licence claim. No licence term is supplied from memory. Source remains an experiment; no runtime binary is redistributed. |
| Local source for the Node `vm` security warning? | **not known here**. The packet states the warning. Independently, the local constructor probes below demonstrate host access in this unchecked VM configuration; no security-boundary claim is made. |

## Bench issues

The corpus and bench are unchanged. B10 is the already recorded corpus issue in
`work/experiments/language-lx/lx3-sandbox-flow/part-b/CORPUS-ISSUES.md`. Supported
cells reach admission and record `not_run: corpus_issue`. An unsupported cell
returns its cell-level reason before admission in `harness/matrix.py`, so its B10
key remains `not_run` with that reason instead of the literal `corpus_issue`.
The candidate cannot change that precedence. No key is removed, and neither
reason is counted as a pass. No workaround or corpus version change was made.

## Production, acceptance and handoff

Commands are from the repository root; paths after matrix flags are relative to
`work/experiments/language-lx/lx3-sandbox-flow/part-b/`.

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-node/prepare.py
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-node/prepare.py --series 10
python -B work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-node/prepare.py --seal
python work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt2-node/check.py
```

Preparation is explicit artifact production; it never runs during acceptance.
`--series` invokes the shared matrix, retains the summary and per-key outcomes,
records raw-output digests and removes its build directory. Acceptance verifies
the corpus's raw freeze and file hashes without requiring today's chapters to
match historical document digests. It verifies candidate and tool pins,
regenerates programs byte for byte, repeats local probes, then runs:

```text
python work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt2-node --out candidates/rt2-node/build/check --runs 1 --leg Windows
```

The fresh keys, payload hashes and recorded outcomes must match the retained
series. A reproduced failure of a form does not fail acceptance. A missing
outcome, changed pin or irreproducible result does. `build/` is removed even on
failure, and all candidate file hashes are compared before and after; reports,
summaries and digests are never refreshed by the check.

Raw series regeneration (output folder must be absent):

```text
python work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/rt2-node --out candidates/rt2-node/build/Windows --runs 10 --leg Windows
```

No Linux leg is possible in the supplied environment: WSL has no Node. It is
**not run here**, and no installation is authorized. The RTE.46 twenty-run
Windows/Linux criterion remains open; it is not inferred from Windows repeats.
No GPU, native regression or timing command is required by this packet. The
integrator owns independent review and any later cross-platform scope. If the
integrator reruns on Windows outside this sandbox, the exact acceptance command
is the `check.py` command above; it requires the pinned tools. No other external
execution is required to reproduce the Windows result.

Read sources: `AGENTS.md`, `docs/development-guide/AGENT_BRIEFING.md`, chapter 11
`docs/architecture/1.0/language/architecture/11-route-and-spikes.md` (RTE.11,
RTE.12, RTE.24 to RTE.27, RTE.30 to RTE.37, RTE.44 to RTE.46), the owner allowance,
`work/experiments/language-lx/README.md`, the frozen corpus notation/kernel/policy/
fixtures and key/freeze files, the bench interface and modules, both controls,
and the bench's `RESULT.md` and `CORPUS-ISSUES.md`. Moving chapter and decision
files are named here, not pinned as acceptance dependencies.

## Effort and limits of inference

Effort was about **1.2 agent-hours elapsed** in this wrapper call, including
repeat-run waiting and acceptance. This is not a product-effort estimate.

One implementation wrapper call built the mechanical renderer, strict subset
recognizer, static data/control analysis, persistent Node bridge, byte bindings,
local escape probes and write-free acceptance. Shared policy, exact arithmetic,
tariffs, fake kernel, host provenance, classifier and matrix/report format were
reused. The generated JavaScript format, adapter, checker and synthetic probes
are throwaway experiment infrastructure. They are not a product front end,
runtime security layer, editor path or estimate of product engineering effort.

An initial ten-repeat production attempt reached the producer's 900-second
aggregate limit while enumerating unsupported cells. Its incomplete outputs were
removed; it is not a passing series. The producer limit was raised to 1,800
seconds and the complete series was rerun under refreshed candidate pins.
Candidate VM, per-child, logical-budget and acceptance limits were unchanged.
The first acceptance invocation also caught a local-helper import-name collision
before running the matrix. The import was corrected. Runtime sources are pinned
in the run binding; producer, probe and acceptance sources are pinned in the
candidate manifest. The final series below was regenerated for that binding.

The card cannot show **all-program safety, authorship, understanding, absence of
covert channels, product latency or an acceptable false-refusal threshold**.
This run also cannot establish cross-platform exactness, real-engine correctness,
OS sandbox containment, Windows/DirectX, input, long-session behaviour or speed.
The static stop/emission failures remain failures, and the unchecked VM escapes
remain escapes. No route or flow mechanism is selected.

## Retained series and per-cell observations

Windows ran **10 repetitions per key and host condition**,
recording **5220 observations** in all six declared cells. Each repetition
uses fresh case data; sequential runs retain only the defined host workspace
and pending preview. These include explicit unsupported-cell outcomes.
No twenty-run Windows/Linux series is claimed.

Raw `bindings.json` SHA-256: `66b9e6b28bbb57791089b2449e707d489fc8313933d5ac0701e7e5c08c3c4969`.
Raw `summary.json` SHA-256: `b93594c746fa5435193530b55f6858b9cdb2502d257a348f1b12bc515268b5f1`.
The per-file pins are in `MANIFEST.json`; raw-series digests are in `series-evidence.json`.
The exact key/payload pairs for every invocation are in `summary.json` and `key-outcomes.json`.
Large raw JSONL is removed; the production command above regenerates it.

### capability_separation / source_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | `not_run`; RT2 requires the approved_subset_byte_bound variant; source_bound not implemented |
| Must-refuse cases | not_run; 0 tested, no passing count |
| Wrong codes | not measured |
| Allowed tools / false refusals | 35 retained keys; 0 tested, false-refusal threshold not applied |
| Refused programs and codes | None; every invocation is not_run |
| Planted reads | not_run; no security inference |
| Budget / cache / interruption | All not_run; B10 also has the frozen corpus issue (see Bench issues) |
| Reproducibility counterexamples | 0; only not_run payloads repeat |
| Crashes / hangs | 0 / 0; no candidate program executed |

### capability_separation / approved_subset_byte_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | `supported` |
| Must-refuse passes | 20 / 20; preparation runs remain separately recorded |
| Must-refuse failures | 0 |
| Wrong codes | must-refuse: 0; planted: 0; approval: 0 |
| Allowed tools | 33 / 35 completed with the frozen values and observations |
| False refusals | 2 / 35; `A25` `LG3:capability.not_granted`, `A30` `LG3:capability.not_granted` |
| Planted reads | `P01`: refused_expected; `P02`: refused_expected; `P03`: refused_expected; `P04`: refused_expected; 0 classified planted reads |
| Approval probes | `S01`: refused_expected; `S02`: refused_expected |
| Reproducibility counterexamples | 0 |
| Crashes / hangs | 0 / 0 |
| Refused programs: `KN7:input.budget_not_declared` | `B13/r1`; layer `rt2_node` |
| Refused programs: `KN7:resource_limit.budget_ceiling` | `B14/r1`; layer `rt2_node` |
| Refused programs: `LG3:capability.ambient_read` | `P01/r1`, `P02/r1`, `P03/r1`; layer `shared_policy` |
| Refused programs: `LG3:capability.not_granted` | `A25/r1`, `A30/r1`, `MR01/r1`, `MR02/r1`, `MR03/r1`, `MR04/r2`, `MR05/r1`, `MR06/r1`, `MR07/r2`, `MR09/r1`, `MR10/r1`, `MR11/r1`, `MR12/r1`, `MR13/r1`, `MR14/r1`, `MR15/r1`, `MR16/r1`, `MR17/r1`, `MR18/r1`, `MR19/r1`, `MR19/r2`, `MR19/r3`, `MR20/r1`, `MR20/r2`, `MR20/r3`, `P04/r1`, `S01/r1`; layer `shared_policy` |
| Refused programs: `LG5:input.module_digest_mismatch` | `S02/r1`; layer `rt2_byte_approval` |
| Refused programs: `PS1:capability.provenance_incompatible` | `MR08/r2`; layer `shared_policy` |
| `B01/r1/default` | `budget_ok`; code `KN7:budget.steps`; steps/kernel/memory/output `2/0/0/0`; stop `r1/body/2:before_step` |
| `B02/r1/default` | `budget_ok`; code `KN7:budget.kernel_cost`; steps/kernel/memory/output `2/3/7/0`; stop `r1/body/1:before_kernel` |
| `B03/r1/default` | `budget_ok`; code `KN7:budget.memory`; steps/kernel/memory/output `2/0/16/0`; stop `r1/body/1:before_memory` |
| `B04/r1/default` | `budget_ok`; code `KN7:budget.output`; steps/kernel/memory/output `2/0/0/6`; stop `r1/body/1:before_output` |
| `B05/r1/default` | `budget_ok`; code `KN7:budget.kernel_cost`; steps/kernel/memory/output `1/0/0/0`; stop `r1/body/0:before_kernel` |
| `B06/r1/default` | `budget_ok`; code `KN7:budget.steps`; steps/kernel/memory/output `0/0/0/0`; stop `r1/body/0:before_step` |
| `B07/r1/default` | `budget_ok`; code `KN7:budget.memory`; steps/kernel/memory/output `4/0/48/0`; stop `r1/recurse:before_frame_4` |
| `B08/r1/default` | `budget_ok`; code `KN7:cancelled.by_user`; steps/kernel/memory/output `3/0/1/0`; stop `r1/body/0:loop_continuation/1` |
| `B09/r1/default` | `budget_ok`; code `KN7:cancelled.by_user`; steps/kernel/memory/output `1/2/117/0`; stop `r1/body/0:chunk/1` |
| `B10/r1/default` | `not_run`; code `null`; `corpus_issue` |
| `B11/r1/default` | `budget_ok`; code `KN7:resource_limit.runtime_lost`; steps/kernel/memory/output `0/0/0/0`; stop `r1/after_admission` |
| `B12/r1/cold` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/warm` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/evicted` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/poisoned` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/shared` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/relocated` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B13/r1/default` | `budget_ok`; code `KN7:input.budget_not_declared`; steps/kernel/memory/output `0/0/0/0`; stop `r1/admission` |
| `B14/r1/default` | `budget_ok`; code `KN7:resource_limit.budget_ceiling`; steps/kernel/memory/output `0/0/0/0`; stop `r1/admission` |

### static_labels / source_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | `not_run`; RT2 requires the approved_subset_byte_bound variant; source_bound not implemented |
| Must-refuse cases | not_run; 0 tested, no passing count |
| Wrong codes | not measured |
| Allowed tools / false refusals | 35 retained keys; 0 tested, false-refusal threshold not applied |
| Refused programs and codes | None; every invocation is not_run |
| Planted reads | not_run; no security inference |
| Budget / cache / interruption | All not_run; B10 also has the frozen corpus issue (see Bench issues) |
| Reproducibility counterexamples | 0; only not_run payloads repeat |
| Crashes / hangs | 0 / 0; no candidate program executed |

### static_labels / approved_subset_byte_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | `supported` |
| Must-refuse passes | 16 / 20; preparation runs remain separately recorded |
| Must-refuse failures | 4; `MR13`: unrefused, `MR14`: unrefused, `MR19`: unrefused, `MR20`: unrefused |
| Wrong codes | must-refuse: 0; planted: 0; approval: 0 |
| Allowed tools | 35 / 35 completed with the frozen values and observations |
| False refusals | 0 / 35 |
| Planted reads | `P01`: refused_expected; `P02`: refused_expected; `P03`: refused_expected; `P04`: refused_expected; 0 classified planted reads |
| Approval probes | `S01`: refused_expected; `S02`: refused_expected |
| Reproducibility counterexamples | 0 |
| Crashes / hangs | 0 / 0 |
| Refused programs: `KN7:input.budget_not_declared` | `B13/r1`; layer `rt2_node` |
| Refused programs: `KN7:resource_limit.budget_ceiling` | `B14/r1`; layer `rt2_node` |
| Refused programs: `LG3:capability.ambient_read` | `P01/r1`, `P02/r1`, `P03/r1`; layer `shared_policy` |
| Refused programs: `LG3:capability.not_granted` | `MR01/r1`, `MR02/r1`, `MR03/r1`, `MR04/r2`, `MR05/r1`, `MR06/r1`, `MR07/r2`, `MR09/r1`, `MR10/r1`, `MR11/r1`, `MR12/r1`, `MR15/r1`, `MR16/r1`, `MR17/r1`, `MR18/r1`, `P04/r1`, `S01/r1`; layer `shared_policy` |
| Refused programs: `LG5:input.module_digest_mismatch` | `S02/r1`; layer `rt2_byte_approval` |
| Refused programs: `PS1:capability.provenance_incompatible` | `MR08/r2`; layer `shared_policy` |
| `B01/r1/default` | `budget_ok`; code `KN7:budget.steps`; steps/kernel/memory/output `2/0/0/0`; stop `r1/body/2:before_step` |
| `B02/r1/default` | `budget_ok`; code `KN7:budget.kernel_cost`; steps/kernel/memory/output `2/3/7/0`; stop `r1/body/1:before_kernel` |
| `B03/r1/default` | `budget_ok`; code `KN7:budget.memory`; steps/kernel/memory/output `2/0/16/0`; stop `r1/body/1:before_memory` |
| `B04/r1/default` | `budget_ok`; code `KN7:budget.output`; steps/kernel/memory/output `2/0/0/6`; stop `r1/body/1:before_output` |
| `B05/r1/default` | `budget_ok`; code `KN7:budget.kernel_cost`; steps/kernel/memory/output `1/0/0/0`; stop `r1/body/0:before_kernel` |
| `B06/r1/default` | `budget_ok`; code `KN7:budget.steps`; steps/kernel/memory/output `0/0/0/0`; stop `r1/body/0:before_step` |
| `B07/r1/default` | `budget_ok`; code `KN7:budget.memory`; steps/kernel/memory/output `4/0/48/0`; stop `r1/recurse:before_frame_4` |
| `B08/r1/default` | `budget_ok`; code `KN7:cancelled.by_user`; steps/kernel/memory/output `3/0/1/0`; stop `r1/body/0:loop_continuation/1` |
| `B09/r1/default` | `budget_ok`; code `KN7:cancelled.by_user`; steps/kernel/memory/output `1/2/117/0`; stop `r1/body/0:chunk/1` |
| `B10/r1/default` | `not_run`; code `null`; `corpus_issue` |
| `B11/r1/default` | `budget_ok`; code `KN7:resource_limit.runtime_lost`; steps/kernel/memory/output `0/0/0/0`; stop `r1/after_admission` |
| `B12/r1/cold` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/warm` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/evicted` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/poisoned` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/shared` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B12/r1/relocated` | `budget_ok`; code `null`; steps/kernel/memory/output `3/6/14/7` |
| `B13/r1/default` | `budget_ok`; code `KN7:input.budget_not_declared`; steps/kernel/memory/output `0/0/0/0`; stop `r1/admission` |
| `B14/r1/default` | `budget_ok`; code `KN7:resource_limit.budget_ceiling`; steps/kernel/memory/output `0/0/0/0`; stop `r1/admission` |

### static_runtime_labels / source_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | `not_run`; RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels |
| Must-refuse cases | not_run; 0 tested, no passing count |
| Wrong codes | not measured |
| Allowed tools / false refusals | 35 retained keys; 0 tested, false-refusal threshold not applied |
| Refused programs and codes | None; every invocation is not_run |
| Planted reads | not_run; no security inference |
| Budget / cache / interruption | All not_run; B10 also has the frozen corpus issue (see Bench issues) |
| Reproducibility counterexamples | 0; only not_run payloads repeat |
| Crashes / hangs | 0 / 0; no candidate program executed |

### static_runtime_labels / approved_subset_byte_bound

| Observation | Recorded result |
| --- | --- |
| Declaration | `not_run`; RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels |
| Must-refuse cases | not_run; 0 tested, no passing count |
| Wrong codes | not measured |
| Allowed tools / false refusals | 35 retained keys; 0 tested, false-refusal threshold not applied |
| Refused programs and codes | None; every invocation is not_run |
| Planted reads | not_run; no security inference |
| Budget / cache / interruption | All not_run; B10 also has the frozen corpus issue (see Bench issues) |
| Reproducibility counterexamples | 0; only not_run payloads repeat |
| Crashes / hangs | 0 / 0; no candidate program executed |

The shared classifier retains every failed form. Static labels miss the four
stop/emission or accumulation cases MR13, MR14, MR19 and MR20. Capability
separation falsely refuses A25 and A30, which combine harmless state facts
with an independent manual or fixed-setup preview. No threshold makes that
cost acceptable automatically. Neither result selects a route.

B08 stops at the first continuation after one completed loop item; B09
stops before the second kernel chunk and preserves the completed first
chunk. All six B12 cache conditions have the same exact payload and logical
charges within a cell. Poisoning adds `LG4:input.cache_entry_mismatch`
only to the excluded host diagnostic envelope. These are synthetic checks.

## Candidate-local adversarial table (outside corpus counts)

The checker is applied first to each source and rejects it. The runtime
column deliberately bypasses that checker in a fresh context using the
same runtime constructor. Escapes read only the public Node version.
They never open files, inspect environment variables, launch a process
or contact a network. No escape probe is added to a corpus denominator.

| Probe source | Checker | Unchecked VM observation |
| --- | --- | --- |
| `tick.constructor.constructor("return process.version")()` | rejected before execution | returned: `v24.21.0` |
| `this.constructor.constructor("return typeof process")()` | rejected before execution | EvalError |
| `Object.getPrototypeOf(tick).constructor("return process.version")()` | rejected before execution | returned: `v24.21.0` |
| `Object.getPrototypeOf(this).constructor.constructor("return typeof process")()` | rejected before execution | EvalError |
| `({}).constructor.constructor("return 7")()` | rejected before execution | EvalError |
| `eval("1+1")` | rejected before execution | EvalError |
| `typeof process` | rejected before execution | returned: `undefined` |
| `typeof require` | rejected before execution | returned: `undefined` |
| `typeof Date` | rejected before execution | returned: `function` |
| `typeof Math.random` | rejected before execution | returned: `function` |
| `while (true) {}` | rejected before execution | timeout |

The actual adapter also returned `KN7:resource_limit.safety_stop` for the local no-host-call loop and `KN7:resource_limit.runtime_lost` after the local Node kill. The unexecuted-branch join and modified-byte
checks passed. Acceptance repeats these local observations without changing
their stored table. The successful unchecked constructor walks mean the
VM itself is not a security boundary in this configuration.
