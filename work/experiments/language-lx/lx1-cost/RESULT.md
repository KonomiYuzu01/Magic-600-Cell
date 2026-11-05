# LX-1: work and boundary traffic on LX-3 prototypes

**Lead only. The decisive LX-1 run waits for LQ8 and frozen experiment versions of the three
LP3 tools. Neither exists. No figure here selects or rejects a route, answers an LQ or establishes E2.**

## CT-RTE-05 record

| Field | Record |
| --- | --- |
| LX card / experiment | LX-1, chapter 11 RTE.38-RTE.40 / E-2.4-L1 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; experiment index: counters on LX-3 prototypes as a lead with the host named |
| Time box | what can be done now; one wrapper call of at most two hours. A reached limit is no pass. |
| Frozen workload/input manifest | `workloads.json` and `INPUTS.json`; one unchanged lx3-neutral-v2 input set; raw FREEZE SHA-256 `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627` |
| Candidate/source scope | Five original LX-3 candidates plus the explicit LX-1 labels-off variants. All original source, artifact, summary and manifest pins are verified. Local sources are separately pinned. |
| Input manifest SHA-256 | `62c278679740585c32caa8dd5a970a31c77ed1ac3345a99231c6dac73d2a46de` |
| Workload file SHA-256 | `bab6afee2ecbc010f3d20e96e31a3910ee1c96d4bbae9ab337712ee5fe4d6fae` |
| Commands / tools | Commands below; CPython 3.15.1; Node 24.21.0; V8 13.6.233.17-node.53; Python standard library and Node built-ins only |
| Host | Windows sandbox; CPython interpreter/host library, Node vm, and Node built-in WebAssembly engine. The experiment index names the machine as AMD Ryzen 9 7945HX / RTX 4070 Laptop GPU; no GPU work ran here. |
| Evidence class | source and synthetic. All timing is a shared-machine sandbox lead, not performance or Windows/DirectX evidence. |
| Kill criteria | Stop a run on output, identity, accounting or source-binding failure. Retain the failure. A slow observation rejects nothing. |
| Actual result | 39 runnable workload/mode cells and six declared not_run cells; 416 program invocations in the count pass. All comparable exact summary hashes and all cross-mode observable result hashes agree. Each runnable cell has a smoke run and three series of 100. |
| Decision / decider | **open / owner**; every route, host, placement, flow-control, syntax and LQ decision remains open |

Original candidate raw `bindings.json` / `summary.json` SHA-256 (verified, not regenerated):

| Candidate | Bindings | Summary |
| --- | --- | --- |
| rt1-own | `c5b2aba702743b907c21a2cab772cafa88ddf0f5880063203e7d56a5e488847d` | `14c38c9ff9f63fce6ee1e4c7b3e484d8bdeb277c61d8f2820537bebcdc782dbd` |
| rt2-py | `77c962966daa2054be7b1647d884a33f2e43dd516ffb8168df8ddf5568830755` | `704ccd9ddf9c07b8a68eaa76baa3fc6b0bc3e8f104fbee4f9b351cf3465e3889` |
| rt2-node | `66b9e6b28bbb57791089b2449e707d489fc8313933d5ac0701e7e5c08c3c4969` | `b93594c746fa5435193530b55f6858b9cdb2502d257a348f1b12bc515268b5f1` |
| rt3-wasm | `3150c75020ec9ebb14ab57659f914a73789b243d16250292e1b41cffd8458a1a` | `235d225b7b63506cac036ba938d9f37b4d4699ef745a49329b5d38933ff4a995` |
| rt4-pyhost | `b036e0991cc9cc0d173282e6bab7eeabfe3ee4a9c488ac08746abe3e54b50a16` | `23b4c743a901d17e9fcee7437e37b24e6aed0c843a561a9a3f37ba21e04d9cce` |

## Workloads and missing real work

Selection was written in `CARD.md` before measurement. `full` is A01-A10, A21-A24, A26-A29;
`jumbling` is A31/A32/A34/A35; `representations` is A11-A20, each in ID order with a fresh host
per program. A25 and A30 cannot run under the capability-separation basis of labels-off and are
excluded. A33 is outside the category/row selection rule. No allowed program calls `bits`.

The Full stand-in has no real full-state kernel or live view. Jumbling uses tiny constant nu arrays,
two-by-two arithmetic, workspace storage and a synthetic research capture; it has no real pose
state or large research workload. Representations uses tiny chart plans and literal explanations,
with no renderer or interactive view. These are not frozen implementations of the three LP3 tools.

## Local labels-off changes

The unchanged static/runtime cells run their original adapters. Adapters were read first: these
execution paths write no files, so originals can run read-only; no candidate copy or preparation
script is needed. Python bytecode writes are disabled, including in child workers. All generated
outputs live under LX-1's `build/` and are removed. No corpus, candidate or harness file is edited.

| Prototype | Exact LX-1 labels-off variant |
| --- | --- |
| rt1-own | In-memory AST variant replaces every Flow construction and join in the checker and executor with one empty Flow singleton. Type/effect traversal and source dispatch remain; there is no label inference or propagation. The independent coarse read-state flag remains. |
| rt2-py | Replaces Analysis initialization with a structural call/effect scan and empty annotations. The original AST approval, renderer, bytecodes and bridge execute; no static label pass runs. |
| rt2-node | Replaces analyze with a reachable-operation/effect scan and empty annotations. Original approved JavaScript and unmodified V8 run through the original bridge. |
| rt3-wasm | Skips label analysis; emits the existing constant-field path with every label field zero, no mask joins and no metadata imports. Keeps inert ABI slots and provenance handles. Local Wasm hashes are retained per program; these modules are not relabelled as the originals. |
| rt4-pyhost | The capability cell already skips static analysis. Erases state/computed tag argument expressions and replaces Library knowledge/make label unions with empty labels while retaining read-state; original rendered Python bytecodes execute. |

All five use a local Host that removes observational labels, including preview's label unions.
Capabilities, profile/parts/declassifier provenance, coarse read-state, source approval and the
original logical meter remain. Empty label fields and other scaffolding remain, so labels-off
is not a claim about the cheapest possible implementation. Each local variant has a distinct
binding identity. This experiment supplies no new enforcement or soundness evidence.

RT2 Python/Node `run_time` stays not_run because RTE.12 requires a modified VM. RT4 `run_time`
runs the original **incomplete wrapper-only attempt**, whose native branch-label failures remain
documented in its LX-3 report. This workload does not repair or retest those negative cases.

## Work and bytes (within each prototype only)

**Largest limitation, applying to every figure below:** the fake kernel has four slots. Its
state/nu rows return fixed small arrays; `word_check` does real permutation work only on those
four toy slots. There is no full-model per-sticker kernel work. Figures measure prototype
interpreter, checking, provenance and bridge overhead; they cannot establish batching benefits
on the real model. The source's toy `word_check` qualifies the packet's description of constant rows.

Canonical argument/result bytes use the same compact sorted-key UTF-8 JSON encoding of data
for every prototype, separately in each direction. They exclude provenance and framing, and
are a virtual encoding for in-process calls. Actual bridge bytes include LF, source/Wasm
launch, every helper call and reentrant fuel messages over Node stdio. That wider bridge
often transports handles rather than kernel data; its bytes are not the canonical-byte figure.

The bench column is **continuation calls / sum of continuation weights**, not executed steps,
DET.20's abstract budget or an LG4.8 charge proposal. Python steps are opcode trace events in
rendered code objects only via sys.monitoring.INSTRUCTION, including their module setup;
RESUME and CACHE emit no instruction events. Host/library/harness bytecodes and
compiler work are excluded. V8/Wasm instruction counts are unavailable without a different
engine/instrumentation. Their coarse callbacks/imports are named separately and never steps.

### rt1-own

| Workload | Mode | Executed form | Steps | Coarse unit | Bench continuations | Kernel calls | Canonical args / results B | Actual bridge sent / received B |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | typed IR instructions dispatched by rt1_vm.Machine | 41 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | labels_off | typed IR instructions dispatched by rt1_vm.Machine | 16 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | labels_off | typed IR instructions dispatched by rt1_vm.Machine | 27 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |
| full | static_only | typed IR instructions dispatched by rt1_vm.Machine | 41 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | static_only | typed IR instructions dispatched by rt1_vm.Machine | 16 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | static_only | typed IR instructions dispatched by rt1_vm.Machine | 27 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |
| full | run_time | typed IR instructions dispatched by rt1_vm.Machine | 41 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | run_time | typed IR instructions dispatched by rt1_vm.Machine | 16 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | run_time | typed IR instructions dispatched by rt1_vm.Machine | 27 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |

### rt2-py

| Workload | Mode | Executed form | Steps | Coarse unit | Bench continuations | Kernel calls | Canonical args / results B | Actual bridge sent / received B |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | CPython 3.15.1 opcode events in rendered code objects | 548 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | labels_off | CPython 3.15.1 opcode events in rendered code objects | 212 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | labels_off | CPython 3.15.1 opcode events in rendered code objects | 380 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |
| full | static_only | CPython 3.15.1 opcode events in rendered code objects | 548 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | static_only | CPython 3.15.1 opcode events in rendered code objects | 212 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | static_only | CPython 3.15.1 opcode events in rendered code objects | 380 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |
| full | run_time | CPython 3.15.1 opcode events in rendered code objects | not_run | Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified | not_run | not_run | not_run | not_run |
| jumbling | run_time | CPython 3.15.1 opcode events in rendered code objects | not_run | Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified | not_run | not_run | not_run | not_run |
| representations | run_time | CPython 3.15.1 opcode events in rendered code objects | not_run | Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified | not_run | not_run | not_run | not_run |

### rt2-node

| Workload | Mode | Executed form | Steps | Coarse unit | Bench continuations | Kernel calls | Canonical args / results B | Actual bridge sent / received B |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | JavaScript on unmodified V8; instruction count not available here | not available here | 64 executed JavaScript domain-helper callbacks | 65 / 65 | 14 | 59 / 473 | 16825 / 5429 |
| jumbling | labels_off | JavaScript on unmodified V8; instruction count not available here | not available here | 28 executed JavaScript domain-helper callbacks | 93 / 93 | 4 | 8 / 20 | 5658 / 2351 |
| representations | labels_off | JavaScript on unmodified V8; instruction count not available here | not available here | 44 executed JavaScript domain-helper callbacks | 39 / 39 | 6 | 31 / 367 | 11156 / 3681 |
| full | static_only | JavaScript on unmodified V8; instruction count not available here | not available here | 64 executed JavaScript domain-helper callbacks | 65 / 65 | 14 | 59 / 473 | 16825 / 5429 |
| jumbling | static_only | JavaScript on unmodified V8; instruction count not available here | not available here | 28 executed JavaScript domain-helper callbacks | 93 / 93 | 4 | 8 / 20 | 5658 / 2351 |
| representations | static_only | JavaScript on unmodified V8; instruction count not available here | not available here | 44 executed JavaScript domain-helper callbacks | 39 / 39 | 6 | 31 / 367 | 11156 / 3681 |
| full | run_time | JavaScript on unmodified V8; instruction count not available here | not_run | RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels | not_run | not_run | not_run | not_run |
| jumbling | run_time | JavaScript on unmodified V8; instruction count not available here | not_run | RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels | not_run | not_run | not_run | not_run |
| representations | run_time | JavaScript on unmodified V8; instruction count not available here | not_run | RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels | not_run | not_run | not_run | not_run |

### rt3-wasm

| Workload | Mode | Executed form | Steps | Coarse unit | Bench continuations | Kernel calls | Canonical args / results B | Actual bridge sent / received B |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | generated WebAssembly; instruction count not available here | not available here | 208 executed Wasm-to-host import calls | 65 / 65 | 14 | 59 / 473 | 23431 / 7923 |
| jumbling | labels_off | generated WebAssembly; instruction count not available here | not available here | 158 executed Wasm-to-host import calls | 93 / 93 | 4 | 8 / 20 | 8433 / 6299 |
| representations | labels_off | generated WebAssembly; instruction count not available here | not available here | 147 executed Wasm-to-host import calls | 39 / 39 | 6 | 31 / 367 | 13986 / 5379 |
| full | static_only | generated WebAssembly; instruction count not available here | not available here | 208 executed Wasm-to-host import calls | 65 / 65 | 14 | 59 / 473 | 23431 / 7971 |
| jumbling | static_only | generated WebAssembly; instruction count not available here | not available here | 158 executed Wasm-to-host import calls | 93 / 93 | 4 | 8 / 20 | 8433 / 6327 |
| representations | static_only | generated WebAssembly; instruction count not available here | not available here | 147 executed Wasm-to-host import calls | 39 / 39 | 6 | 31 / 367 | 13986 / 5405 |
| full | run_time | generated WebAssembly; instruction count not available here | not available here | 237 executed Wasm-to-host import calls | 65 / 65 | 14 | 59 / 473 | 24469 / 8812 |
| jumbling | run_time | generated WebAssembly; instruction count not available here | not available here | 167 executed Wasm-to-host import calls | 93 / 93 | 4 | 8 / 20 | 9010 / 6589 |
| representations | run_time | generated WebAssembly; instruction count not available here | not available here | 164 executed Wasm-to-host import calls | 39 / 39 | 6 | 31 / 367 | 14686 / 5898 |

### rt4-pyhost

| Workload | Mode | Executed form | Steps | Coarse unit | Bench continuations | Kernel calls | Canonical args / results B | Actual bridge sent / received B |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | CPython 3.15.1 opcode events in rendered code objects | 990 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | labels_off | CPython 3.15.1 opcode events in rendered code objects | 418 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | labels_off | CPython 3.15.1 opcode events in rendered code objects | 773 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |
| full | static_only | CPython 3.15.1 opcode events in rendered code objects | 990 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | static_only | CPython 3.15.1 opcode events in rendered code objects | 418 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | static_only | CPython 3.15.1 opcode events in rendered code objects | 773 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |
| full | run_time (incomplete wrapper attempt) | CPython 3.15.1 opcode events in rendered code objects | 990 | not applicable | 65 / 65 | 14 | 59 / 473 | no transport |
| jumbling | run_time (incomplete wrapper attempt) | CPython 3.15.1 opcode events in rendered code objects | 418 | not applicable | 93 / 93 | 4 | 8 / 20 | no transport |
| representations | run_time (incomplete wrapper attempt) | CPython 3.15.1 opcode events in rendered code objects | 773 | not applicable | 39 / 39 | 6 | 31 / 367 | no transport |

## Kernel mix and size projections

Only calls through `ctx.kernel_call` are counted here. Preview's internal fake check is a
host operation outside that boundary. No selected case calls batch_check. Every runnable
prototype/mode produced the following same mix; zeros are retained. Full details by row and
direction, including per-program result hashes, are in `counters.json`.

| Kernel row | full calls | jumbling calls | representations calls |
| --- | --- | --- | --- |
| batch_check | 0 | 0 | 0 |
| bits | 0 | 0 | 0 |
| blocked | 1 | 0 | 0 |
| caps | 0 | 0 | 0 |
| counts | 3 | 0 | 2 |
| floors | 2 | 0 | 1 |
| nu2 | 0 | 3 | 0 |
| nu5 | 0 | 1 | 0 |
| preview_outcome | 0 | 0 | 0 |
| score | 0 | 0 | 0 |
| setup | 4 | 0 | 0 |
| snapshot | 1 | 0 | 0 |
| word_check | 3 | 0 | 3 |

**Projection, never measurement.** RTE.33 supplies sizes, not a mapping from nu arrays to poses.

| Workload | snapshot calls x 1,039,200 label B | Pose bundle size | Pose projection |
| --- | --- | --- | --- |
| full | 1 x 1,039,200 = 1,039,200 | 869,952 B | not available here: no pose-transfer row |
| jumbling | 0 x 1,039,200 = 0 | 869,952 B | not available here: no pose-transfer row |
| representations | 0 x 1,039,200 = 0 | 869,952 B | not available here: no pose-transfer row |

LX-2 can use this per-row stand-in mix and the explicit label projection. It cannot infer
a pose-transfer frequency from this corpus. No full-size payload was transferred by LX-1.

## Timing and equality

`SERIES.md` reports median and p99 of all three 100-run series per cell, including time per
executed step where available and per kernel call. `timings.json` retains ordered raw samples
and smoke runs. Times are whole-workload elapsed divided by counts, not isolated call latency.
The timer and child lifecycle are documented there. D6 conditions were not controlled; other
work shares the machine. All measurements are weak leads and no slow result rejects anything.

For original static/runtime cells, each exact payload hash and full run key matches every
corresponding stored summary repeat (256 program observations, 2,560 stored-repeat comparisons).
All 416 count-pass observations match the frozen allowed-tool oracle. Each workload's observable
result hash agrees across its runnable modes. Local labels-off exact hashes are deliberately
not compared with old capability hashes; their new identity and removed labels differ.

Cross-mode payload normalization is specified in CARD.md and `runner.py:result_payload`: it
retains exact typed outputs, order, coverage, display, effects and protected session state,
excluding form/source identities, budgets and provenance-dependent metadata. This is not a
claim that full LX-3 metadata or hashes are identical across modes. Full hashes remain retained.

## Effort and reuse (RTE.9)

Only rt2-node's report gives elapsed agent hours, including waiting and acceptance. The other
reports do not give hours; do not turn a wrapper's time limit into effort. These rows are
leads and set no product estimate. LX-1 wrapper work is not separately metered by prototype.

| Prototype | Agent hours | Slice built in its LX-3 report | Reused here / throwaway |
| --- | --- | --- | --- |
| rt1-own | not known | Own lexer/parser, type/effect/flow checker, typed IR interpreter and source binding | Original executor and synthetic bench reused; label erasure/count wrapper and experiment surface are throwaway |
| rt2-py | not known | Mechanical Python renderer, approved AST subset, static checker and domain bridge | CPython and original artifacts reused; effect-only variant/opcode observer are throwaway |
| rt2-node | about 1.2 agent-hours elapsed, as its LX-3 report records; includes waiting and acceptance | Canonical JS renderer/subset recognizer, static analysis and stdio handle bridge | Node/V8 and original JS reused; effect-only variant/pipe observer are throwaway |
| rt3-wasm | not known | Own array surface, analysis, handwritten Wasm lowering and host-handle bridge | Node built-in Wasm and original compiler reused; zero-label lowering/import observer are throwaway |
| rt4-pyhost | not known | Python renderer, subset checker and domain library; incomplete runtime wrapper attempt | CPython/library/programs reused; label-free library/opcode observer are throwaway |

The shared policy, arithmetic, provenance, fake kernel, source maps and oracle remain the same.
LX-1 adds fixed selection, observations, explicit local variants and repeatability/report drivers.
No product implementation, renderer, VM modification, installation or external service was built.

## Commands, remaining work and limits

Executed from the repository root:

```text
python -B work/experiments/language-lx/lx1-cost/measure.py --freeze
python -B work/experiments/language-lx/lx1-cost/measure.py --record
python work/experiments/language-lx/lx1-cost/check.py
```

The check verifies corpus/raw/normalized freeze, each candidate's stored source/artifact
pins and source maps, local source pins, and tool versions (also the stored Node/Python
executable pins). It recomputes every count once under `build/`, compares stored counters,
full summary hashes where comparable and normalized results across modes, and removes
`build/` even on failure. It neither measures nor parses timing samples, refreshes pins nor
rewrites reports. A byte snapshot checks retained-file immutability. There is no speed bar.

**Not run here:** optional N=1/10/100/1000 word_check-loop versus batch_check scale contrast.
Priority was the complete frozen-corpus slice. These adapters bind approval to frozen artifacts;
new neutral scale programs need a separately scoped local approval/artifact variant, not edits
to the corpus or existing candidates. No scale count or batching-speed result is claimed.

**Not run here:** Linux, a selected LQ8 host, real/frozen LP3 tools, real pose traffic, a modified
RT2/RT4 VM, foreground renderer, GPU/native input, long-session runs and decisive E2 timing.
For the integrator's repeat of this same Windows lead outside the sandbox, use exactly:

```text
python work/experiments/language-lx/lx1-cost/check.py
python -B work/experiments/language-lx/lx1-cost/measure.py --record
```

Preserve this observation before explicitly recording a new series. These commands remain
prototype leads even on an idle machine; they do not become decisive by moving outside the
sandbox. No truthful decisive-host/LP3/GPU command exists until those candidates and their
inputs are supplied and frozen. Linux would need a new host/tool identity, not a bypass of pins.

Independent integration review remains with the integrator. No nested review-wrapper call
was made: this packet forbids its network use and writes outside the owned folder. No Git
mutation was performed. No rule, source, corpus or candidate outside this folder was changed.

Cannot show: product responsiveness, a route pass, equal instruction counts across routes,
LP3's charter E2 result, real-kernel scaling or a product label-cost estimate. Nothing here
resolves LQ8/LQ9/LQ21 or any other LQ. Negative/unavailable evidence remains explicit.

Sources read: AGENTS.md and the current agent briefing; chapter 11 RTE.9/RTE.12/RTE.24 and
RTE.30-RTE.40; the experiment index and owner allowance; all five LX-3 candidate reports,
declarations, bindings, summaries and adapters; the named harness/corpus files; LX-2 RESULT.md
for the two real sizes. Moving governance sources are named, never acceptance-pinned.
