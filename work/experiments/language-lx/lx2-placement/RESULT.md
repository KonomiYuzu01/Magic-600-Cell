# LX-2 headless result: shared-host native failures remain failures

The separate runtime process satisfied conditions 1 and 2 for all seven injected
fault cases on this frozen candidate. The runtime thread inside the engine process
failed the native allocation, native recursion, native loop and forced-kill cases.
**No placement decision is made. No complete LX-2 pass is established:** conditions
3 and 4, first-frame startup and the shell placement were not run here.

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-2, RTE.41 to RTE.43 / E-2.4-L2, headless slice |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2, and this implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Frozen candidate | Base `ab046f4f8fecf201f0d1cb348b45b29adaead7fb`; source/input identity `228d5064db49c6f47217d06850cea41a16a5436817f3216fc0ac996d0649a2ce` |
| Input and source digests | `CANDIDATE.json`: all local source and input files, reused LX-6 sources, generated model and pose digests; `MANIFEST.json`: frozen file set and results |
| Exported full model | SHA-256 `223edad95f991ea1ada8ed3d464b21909979f0122aa837fc9e3d7e3541949e01`, identical to LX-6's recorded model export |
| Pose transfer fixture | 869,952 bytes; SHA-256 `0d3068ad1aca38646ef4f858b4f98de9eb78c6a54f2fce6128e5642933bc9fb3` |
| Rust executable | SHA-256 `d9580c6d3714648f0013799e48f985f396ca4998a3479194966ffabf77c9a30f`, rebuilt by acceptance and removed |
| Commands and versions | Commands below; CPython 3.15.1; rustc 1.98.1 (`48a229cea`), target `x86_64-pc-windows-msvc`, LLVM 22.1.8; Visual Studio installation 18.10.12224.181, linker 14.51.36260.0 |
| Evidence class | **source and synthetic**: actual compiled experiment kernel with full-model mechanics, fresh journals and injected faults. All timing is a **sandbox lead**, not performance evidence. No Windows/DirectX or native-input claim. |
| Kill criteria | Any failed fault condition, including loss of the session under forced kill, fails that placement's tested slice. Keep the counterexample and control. Continue other cases; never turn a timeout into a pass. |
| Actual result | 5 November 2026: two controls and 14 fault trials completed; all expected durable prefixes remained intact; four shared-host fault cases fail. Both placements completed one smoke run and three 100-sample series per runtime row. |
| Decision / decider | **open / owner**. Every LQ, route, language, flow-control form and placement decision stays open. |

`CARD.md` defines the boundaries, fault limits, fixed commands and interpretation.
`RESULTS.json` retains every correctness field, including outcome codes, journal
row digests, command completions, live state, pending previews and prior receipts.
`TIMINGS.json` retains all raw samples and summary statistics. The complete
259,800-slot model and all 1,200 generators remain unchanged; journal replay is
checked independently in Python. No personal session is read or written.

## Fault/control results

Each table cell is **condition 1 / condition 2**. Condition 1 compares the actual
journal bytes and state hash with the same placement's control. Condition 2
requires a typed terminal outcome and the same open session. Only the explicitly
labelled logical faults are caught by the runtime's own meter.

| Level | Fault | Inside engine | Own process | Observation |
| --- | --- | --- | --- | --- |
| Control | No fault | pass / pass | pass / pass | Same four durable journal rows and final hash |
| Runtime meter | Allocation to 1 MiB | pass / pass | pass / pass | `error`, `budget.memory_bytes`, used = limit = 1,048,576 |
| Runtime meter | Recursion to 64 levels | pass / pass | pass / pass | `error`, `budget.recursion`, used = limit = 64 |
| Runtime meter | Loop without kernel calls | pass / pass | pass / pass | `error`, `budget.instructions`, used = limit = 100,000 |
| Outside meter | Native allocation failure | **fail / fail** | pass / pass | Actual allocator failure aborts shared host; separate host death becomes `runtime.died` |
| Outside meter | Native stack overflow | **fail / fail** | pass / pass | Stack-overflow diagnostic and shared host death; separate host death becomes `runtime.died` |
| Outside meter | Native endless loop | pass / **fail** | pass / pass | Shared runtime thread is still running after the watchdog; separate runtime is killed/reaped with `runtime.watchdog_killed` |
| Outside meter | OS forced kill | **fail / fail** | pass / pass | Shared engine dies; separate engine preserves session and reports `runtime.died` |

The three fatal shared-host counterexamples retain only the previously durable
baseline commit. Their remaining commands could not execute. Their journal is
3,060 bytes, one row, SHA-256
`602bec05e882416bc539f4d8b176232ace4e33e862734c0af7d955853dc5bdd5`, and independently
replays to state hash `392a1b23d7b7819ad9b8845171df1ac7321f9b6735bddc76528c5f582c6e0cb1`.
There is no live session, no available pending preview and no runtime completion
receipt. The driver's offline replay does not count as a restarted/open session.

Their no-fault control has four rows, 12,255 bytes, journal SHA-256
`694a2791eb26c7f8bd488218ad6fa0c1fd7d16b514098a5fa114a5ff7e607c0a`, and final state hash
`4b4560de3f0f62c77ebf053e5a15969dd735ca69fe272113fa60a351d971f17f`. The same complete
journal and hash occur in every surviving command sequence in both placements.
The failed condition 1 results are incomplete command sequences, not evidence of
a corrupted durable prefix. The prefix's prior committed receipt is never
relabeled as a failed commit.

The native-loop shared-host case preserves the journal, state, pending preview and
receipt, and executes the full command tail while the loop continues. It still
**fails condition 2**: no typed terminal outcome exists and the job has not ended.
The fixture process is stopped only after this observation. No restart, detached
replacement session or cooperative-cancellation result hides the failure.

Every separate-process fault retained the engine-owned pending preview and earlier
receipt. The driver could commit that preview, undo it, and complete the remaining
commands. No runtime was restarted. Native allocation and recursion failures were
observed through their actual failure diagnostics; the memory test used a bounded
8 MiB native allocator ceiling, not machine-wide exhaustion. These results do not
test arbitrary allocation pressure or arbitrary crash points inside a journal write.

The independent journal verifier rejected both a changed hash in the second row
and a truncated journal. The runtime's commit attempt and a second pending preview
were refused without a journal change. Full transfer payloads were checked by
length, bytes and digest, and both placements returned identical row outputs.

## Per-row sandbox leads

All values below are **milliseconds**, with 100 samples per row per series. The
separate smoke run has one sample per row and is retained in `TIMINGS.json`.
Median uses the middle pair; p95 and p99 use nearest rank. Largest is the observed
maximum. These are shared-machine sandbox leads, not an E2 or performance pass.

| Placement | Runtime row | Series | Median ms | p95 ms | p99 ms | Largest ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| inside_engine | `read_labels` | 1 | 0.136700 | 0.168600 | 0.187600 | 0.191000 |
| inside_engine | `read_pose_bundle` | 1 | 0.106900 | 0.140100 | 0.147800 | 0.152100 |
| inside_engine | `check_word` | 1 | 0.890250 | 0.958400 | 1.038400 | 1.107800 |
| inside_engine | `preview` | 1 | 3.480650 | 3.625000 | 4.099900 | 4.116100 |
| inside_engine | `read_labels` | 2 | 0.016950 | 0.135300 | 0.155300 | 0.161500 |
| inside_engine | `read_pose_bundle` | 2 | 0.013900 | 0.125400 | 0.145600 | 0.163600 |
| inside_engine | `check_word` | 2 | 0.879800 | 1.010100 | 1.032500 | 1.590400 |
| inside_engine | `preview` | 2 | 3.470900 | 3.572700 | 3.757300 | 3.902800 |
| inside_engine | `read_labels` | 3 | 0.016800 | 0.115100 | 0.140000 | 0.156300 |
| inside_engine | `read_pose_bundle` | 3 | 0.014200 | 0.119400 | 0.137800 | 0.158700 |
| inside_engine | `check_word` | 3 | 0.866200 | 0.952800 | 1.087500 | 1.127700 |
| inside_engine | `preview` | 3 | 3.416350 | 3.542600 | 3.586500 | 4.048400 |
| own_process | `read_labels` | 1 | 0.489450 | 0.713900 | 0.859600 | 0.880900 |
| own_process | `read_pose_bundle` | 1 | 0.415750 | 0.664900 | 0.760800 | 0.766800 |
| own_process | `check_word` | 1 | 0.982150 | 1.269100 | 1.399600 | 1.596700 |
| own_process | `preview` | 1 | 3.690350 | 4.147200 | 4.515800 | 4.965900 |
| own_process | `read_labels` | 2 | 0.755550 | 0.923400 | 1.039700 | 1.072200 |
| own_process | `read_pose_bundle` | 2 | 0.382850 | 0.577500 | 0.636100 | 0.662100 |
| own_process | `check_word` | 2 | 1.003850 | 1.351800 | 1.452600 | 1.569400 |
| own_process | `preview` | 2 | 3.802100 | 4.340500 | 4.680100 | 4.758000 |
| own_process | `read_labels` | 3 | 0.500950 | 0.741400 | 0.880000 | 1.054600 |
| own_process | `read_pose_bundle` | 3 | 0.225000 | 0.575800 | 0.784800 | 0.860600 |
| own_process | `check_word` | 3 | 0.975000 | 1.460200 | 1.662300 | 1.878700 |
| own_process | `preview` | 3 | 3.751550 | 4.226600 | 4.408800 | 4.620300 |

Labels transfer exactly 1,039,200 bytes. Pose transfer is a structured synthetic
869,952-byte bundle with P = 2,049 and one mask, not pose computation by LX-6.
Every preview carries the complete 2,836-byte provenance block in its 2,887-byte
request. The timers include kernel work and response copying; separate-process
timers additionally include request encoding and pipe framing/transfers. State
hashing dominates much of preview's measured interval. These figures do not
establish transport-only overhead, a throughput ceiling or a winning placement.
**LX-1 call mix: open.** No weighted latency or product responsiveness is inferred.

## Commands and remaining work

The recorded production run was:

```text
python -B work/experiments/language-lx/lx2-placement/run.py --output RECORDED.json
```

Its three captured sections are retained as `CANDIDATE.json`, `RESULTS.json` and
`TIMINGS.json`; the temporary combined record was removed. `prepare.py` invokes
`rustc main.rs --edition=2021 -O -D warnings -C debuginfo=0 -C link-arg=/Brepro`
from this folder, with the executable and all temporary products in `build/`.
The exact flags, source hashes and compiler output are in `CANDIDATE.json`.

Acceptance command (the packet's final stop condition is two consecutive passes):

```text
python work/experiments/language-lx/lx2-placement/check.py
python work/experiments/language-lx/lx2-placement/check.py
```

Acceptance reproduces the current candidate's complete correctness record,
including its failed placements. It never compares new timing values or rewrites
the report. It removes `build/` and verifies that the complete folder is byte-identical.
The model and raw journals are generated afresh; the final frozen folder contains
only source, inputs, digests and small records. No LX-6 file was changed.

Integrator work outside this sandbox, in separate checkouts/copies so frozen
records remain intact:

1. **Not run here: WSL/Linux leg**, with an already installed Rust toolchain and
   Python 3.12 or newer. The command captures Linux correctness and separate
   platform/build metadata without asserting equality in advance:

   ```text
   python3 work/experiments/language-lx/lx2-placement/run.py --output LINUX.json
   ```

2. **Not run here: quiet owner-machine repeat of the per-row leads.** Record D6
   power source, Windows power mode, vendor performance mode, driver V-Sync and
   overlays alongside the output before interpreting timing. Run:

   ```text
   python work/experiments/language-lx/lx2-placement/run.py --output WINDOWS-QUIET.json
   ```

   This still supplies only per-row timings, not an E2 or renderer gate result.

3. **Not run here: RTE.42 condition 3.** Needs a renderer candidate actually
   presenting frames, the full renderer workload/gate, a foreground window and
   LX-4's echo measurement during each fault window.
4. **Not run here: RTE.42 condition 4.** Needs that integration plus native input,
   at least 100 commits during each repeated fault window, and full key-to-first-
   correct-frame E2 measurement (p95 at most 100 ms).
5. **Not run here: first-frame startup.** Needs an existing session, loaded runtime,
   a renderer and foreground window; three cold process-start-to-first-interactive-
   frame observations against the still-proposed E3 bar of 3 s.
6. **Not run here: shell-process placement.** Needs an actual shell/renderer host
   with its shared failure domain and the same complete fault/input measurements.

There is no runnable command for items 3 to 6 in this headless slice. Their native
integration harness and exact commands are **open**, not fabricated or replaced
by a headless command. The LX-1 mix, product limits, native resource pressure,
long-session evidence and independent integration review also remain open.

The card's **Cannot show** remains: containment for untested hosts or faults, a
general process-isolation proof, or an accepted E3 budget. In particular, this
tests a Rust thread candidate, not every possible engine isolate or borrowed VM.
The owner remains the decider for every route, placement and language question.
