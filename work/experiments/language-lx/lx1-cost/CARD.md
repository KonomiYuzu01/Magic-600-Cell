# LX-1 prototype cost lead: frozen card

Experiment **E-2.4-L1**, LX-1, chapter 11 RTE.38 to RTE.40. Decision **open**;
decider **owner**. Allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`
and the experiment index: counters on the LX-3 prototypes, with the host named.
Time box: **what can be done now; one wrapper call of at most two hours**.

The decisive run waits for LQ8 and frozen experiment versions of the three LP3
tools; neither exists. These are stand-ins on the frozen LX-3 corpus, not those
tools. No route ranking, cross-route step comparison, E2 claim or LQ decision.

## Selection rule (written before measurement)

Read `lx3-sandbox-flow/corpus/allowed-tools.json` in increasing case-ID order.
Each workload runs its listed programs in that order, once each, with a fresh
host per program and the original input values. Do not combine their source or
share a workspace across programs. Exclude A25 and A30: every prototype refuses
them under capability separation, the basis of the labels-off variant.

* `full`: all cases in categories `reading_state`, `exact_counting`,
  `own_word_checking`, `named_target_setup`, after those exclusions.
  Thus A01-A10, A21-A24, A26-A29 (18 programs).
* `jumbling`: all remaining allowed programs containing a kernel instruction
  for `nu2`, `nu5` or `bits`, including nested bodies/modules.
  Thus A31, A32, A34, A35 (4 programs). No allowed case calls `bits`.
* `representations`: all cases in categories `charts` or `explaining`.
  Thus A11-A20 (10 programs).

A33 is a bounded research search, outside these selection rules. No corpus case
is added or changed. `workloads.json` freezes the lists, program and input-value
digests; `INPUTS.json` freezes that file, this card, the runner sources, corpus,
candidate files and harness. Moving chapters/decision pages are never pinned.

Full lacks full-size state, the real kernel, the live tool and its view. Jumbling
has only tiny fixed nu arrays, two-by-two arithmetic and a synthetic workspace /
capture; no real jumbling state, pose bundle, search scale or real tool. The
representations stand-in publishes tiny plans and explanations, with no renderer,
interactive view or real representation tool. All use the same four-slot fixture.

## Modes and executed units

`static_only` and `run_time` execute the candidates' original static-label and
static/runtime-label cells. RT2 Python/Node runtime modes are `not_run`: an
unmodified VM does not supply RTE.12's runtime labels. RT4 runtime is explicitly
an **incomplete wrapper-only attempt**, not a modified host.

`labels_off` is an LX-1-only variant of capability separation. `variants.py`
removes static and runtime label computation, retaining structural approval,
types/effects where present, capabilities, coarse read-state/profile provenance,
the fake kernel, source bytes and the original logical meter. Empty label fields
and other ABI/provenance scaffolding remain; this is not a minimal runtime.
The report specifies each transformation. Original files remain read-only.

Count RT1 dispatched typed IR instructions at `ctx.executed`; count RT2 Python
and RT4 Python instruction events only in rendered code objects, using
`sys.monitoring.INSTRUCTION`. RESUME and CACHE do not emit those events; host
helpers, compilation and the harness are excluded from these counts. V8 supplies
no instruction counter here. For RT3,
count executed Wasm-to-host import calls under that name, never as Wasm steps;
retain unavailable instruction counts explicitly. RT2 Node helper callbacks are
also recorded as a coarse unit, not JavaScript/V8 instructions. Each report table
containing steps repeats the executed form and has only one prototype.

The unchanged LX-3 meter is separately named `bench_continuation_weight` (sum of
weights), plus `bench_continuation_calls`. Neither is an LX-1 step, a DET.20
abstract meter decision or an LG4.8 charge schedule proposal.

## Boundaries and comparison

Count calls exactly at `ctx.kernel_call`, by row. Canonical bytes are the UTF-8
lengths of compact, sorted-key, non-ASCII-escaped JSON of the ordered argument
**data** array and returned **data**, separately. No type, provenance, framing or
transport metadata is included; this is one common virtual encoding, not a claim
that in-process calls serialize. The real Node bridge counts actual binary stdin
and stdout bytes, including LF, module/source launch and all helper/fuel messages.
It is a wider boundary than kernel data alone. No real transport exists for the
three in-process Python executors; their actual transport byte field is null.

Project `snapshot` calls times 1,039,200 bytes only as a full-label projection.
RTE.33's pose bundle is 869,952 bytes, but this corpus has no pose-transfer row:
retain that size and an unavailable projection, never equate nu arrays with poses.

Compare full exact payload hashes and run keys against all stored summary repeats
for unchanged static/runtime cells. Labels-off has a new local implementation,
so no original exact hash is comparable. Across modes compare an explicit result
payload: outcome/code, typed value, words, certificates, checker reports, coverage,
stop, display, ordered observable effects, session, pending word/target and stored
typed data. Exclude form/source-specific identities, budget values, context layer,
provenance blocks and the pick record's producer digest (which hashes provenance).
Keep the other pick fields. Validate every program against the frozen allowed-tool
oracle as well. Full original hashes remain retained separately; no metadata
normalization is presented as full LX-3 exact-payload equality.

## Timing and stop

One smoke run, then three consecutive series of 100 complete workload runs per
runnable cell. Timer: Python `time.perf_counter_ns`; fresh hosts per program in
one Python worker per candidate/mode. RT2 Node reuses its child with a fresh VM
context for each program; RT3 starts a fresh Node child per program, as its adapter
requires. Preparation, approval, compilation, execution and bridge overhead are
inside the timed workload; Python worker startup, final payload hashing and
result validation are outside. Counting hooks/opcode tracing are off for timings.
Each sample's elapsed time divided by the deterministic cell's executed steps
or kernel calls is an amortized cost, not an isolated instruction/call latency.
Median is the middle-pair mean; p99 is nearest rank (99th of 100).

`measure.py --record` records counts and timings. Acceptance runs counts once
only; it never reads timings, rewrites a report or refreshes a pin. It uses only
`build/`, removes it on success/failure and compares retained folder bytes.
No numeric figure is a rejection criterion. A failed identity, accounting,
outcome or source-binding check stops that run and is retained as a failure.
A reached time limit is no pass. Stop after acceptance exits zero and the report
accounts for all 45 workload/prototype/mode cells. Optional scale contrast waits
until this complete slice is finished and may remain explicitly unrun.
