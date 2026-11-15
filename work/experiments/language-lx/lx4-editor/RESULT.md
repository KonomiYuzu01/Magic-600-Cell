# LX-4 headless result: E-2.4-L4

Decision: **open**; decider: **owner**. No route, engine language, syntax, ADR
or LQ answer is selected. This is experiment code, never product code.

**The headless slice ran. These are function timers, a lead only. RTE.49's
key-message-to-presented-frame bars are printed as context in `SERIES.md` and
are not compared. No figure here passes or fails a bar.**

## Record and identity

| CT-RTE-05 field | Recorded value |
| --- | --- |
| LX card | LX-4, chapter 11 RTE.47 to RTE.53; unchanged card and scoped method in `CARD.md` |
| Experiment ID | E-2.4-L4 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2 and its LQ18 reading; packet authorizes this offline headless slice |
| Time box | **what can be done now; one wrapper call of at most two hours**; a reached limit is no pass |
| Base | `ab046f4f8fecf201f0d1cb348b45b29adaead7fb`; packet-owned worktree only |
| Run | 5 November 2026, Windows 11, shared sandboxed machine |
| Evidence class | source and synthetic; function timings are leads, not performance/gate evidence |
| Kill criteria | Frozen-input/IR/diagnostic/replay disagreement prevents timing; unexpected per-sample outcomes invalidate the complete run; no slow series is truncated. Full RTE.53 criteria are unchanged and untested here |
| Actual result | Both hosts agree in preflight; all 50 function jobs completed one smoke and three 100-sample series, with no unexpected sample outcome |
| Decision and decider | **open**, **owner** |

All hashes below are SHA-256. `MANIFEST.json` contains the individual source and
input hashes, generated module hashes, fragment census and import graph.
Architecture, card and decision pages are named in `CARD.md`, not pinned in a
way that prevents later checks when those documents move.

| Identity | SHA-256 |
| --- | --- |
| LX-4 candidate sources and frozen small artifacts | `0c781e19dba58e3409dac0fad1a02b2209e474a0d4e22264c7188ddd56099787` |
| Read-only LX-8 input/source aggregate | `5598603122fcb84ef1fd2c501464637ca6d0f01fe576572aa7c9483c6d5245df` |
| Generated module-set manifest | `ed9f0f102e989e9389f274cfcdb2fc78a72dbf9b23f0e209af1d00bbcd01d61b` |
| Seeded edit script | `48c2c26f103422989785e61d39e38f2a2be4788f12f3497c5983034e2806b827` |
| Rust executable actually timed, then deleted | `6a1f5131348d4be11a71b9646a3aabc0dde7e32444caf6ff3d93f3912aebd98e` |
| Raw observations, `timings.json` | `c4909e6afbd7b78b9844ba5c19401cd05393691a3608185992471b5cf144a575` |

Hosts: **CPython 3.15.1 running the unchanged LX-8 candidate-a parser and
checker**, and **Rust 1.98.1 standard-library-only candidate-a port**.
`rustc -vV` reported commit `48a229ceaefd4985c50990b14116b6d856af0985`,
host `x86_64-pc-windows-msvc`, LLVM 22.1.8. The compiler flags were
`--edition=2021 -C opt-level=3 -C debuginfo=0`. No crate, registry, installation,
network, real kernel, personal session, renderer or foreground window was used.

The experiment index describes the machine as an AMD Ryzen 9 7945HX with an
RTX 4070 Laptop GPU. Those are context, not GPU measurements. Power source,
Windows power mode, vendor mode, driver V-Sync and overlays were not recorded
or controlled. The machine was shared. Hosts ran sequentially, Python then
Rust, with S/M/L/W jobs in that order; this was not a counterbalanced comparison.

## Frozen inputs and source results

**RTE.51 step 1 asks for the three LP3 tools. Their experiment versions are not
frozen. This module set is an LX-8 stand-in and must be regenerated from those
three tools when they freeze.** It is not their completed corpus or a prediction
of their cost. All 17 LX-8 programs contribute source fragments; deterministic
renaming preserves strings, metadata, local calls and ordered operands.

| Size | Modules | Lines | UTF-8 bytes |
| --- | ---: | ---: | ---: |
| S | 1 | 200 | 23,305 |
| M | 1 | 1,000 | 129,694 |
| L | 1 | 5,000 | 640,941 |
| W | 50 | 20,000 | 2,496,407 |

The retained 5,000-digit integer and other repeated fragments make bytes per
line larger than the research census's approximately 47; equal line counts
do not establish equal workloads. W has 129 synthetic import edges across
base utilities, domain helpers, workflows and entry modules. Imports have
digest-pinned signatures and actual typed call sites. Large modules are generated
under `build/` and removed. Only the 200-line S sample is retained.

Candidate-a was picked because its grammar avoids layout and pipeline handling
in the measured parser. **This is no preference about syntax.** Rust's b/c
grammar adapters let validation consume all 48 original broken drafts; only
candidate-a is timed. The signature/type catalogue is shared frozen data;
Rust independently implements lexing, parsing, recovery and checking. It uses
JSON-shaped trees and prototype allocation patterns, not a production compiler
data layout. Integer and SHA-256 routines reuse the repository's LX-5 experiment
source as documented in `CARD.md`; there is no third-party library.

The current candidate's measurement preflight passed these source checks:

- LX-8's own acceptance: 17 programs in three spellings (51 golden IR checks),
  48 broken drafts, all six syntax codes, eight type families and 45 semantic
  mutation controls. LX-8 was read only, including during imports.
- Exact Rust/Python equality for all 51 complete parsed trees and IRs, all
  48 broken drafts, and 48 Unicode-comment-shifted drafts. Diagnostic codes,
  messages, byte spans, scalar columns, recovery holes and semantic refusals
  agree. No source field is erased in the cross-host comparison.
- Fourteen additional cross-host source mutations: four observable valid IR
  changes and ten semantic refusals, including mismatched types, erased effects,
  missing command authority, forbidden view effects and unavailable imports.
- All 53 generated modules agree and check successfully. S contains all 17 seed
  programs. All 16 seeded edits replay to their frozen digests; stale baselines
  are refused. Completion and syntax-break drafts retain holes and cannot lower.
- W00's exported-signature edit is valid. Its new interface is pinned into
  direct imports; all 49 transitive dependents are rechecked. The 17 direct
  dependents produce the expected type refusals; the remaining 32 check.

Expected negative results are preserved as refusals, not successful programs.
No unexpected equivalence or sample-outcome failure occurred. A separate
forced-exception control created a partial file inside `build/`, raised an
exception, and confirmed cleanup removed the entire build while all remaining
paths and file bytes stayed identical.

## Function observations

[`SERIES.md`](SERIES.md) contains **all 150 series rows**: median, p95, p99 and
largest for each host/size/function, with all three 100-sample series shown.
It prints all six proposed RTE.49 bar rows and the W context beside the figures,
explicitly **not compared**. [`timings.json`](timings.json) retains all **15,000
timed samples and 50 smoke samples** as integer nanoseconds, job input digests,
expected refusal counts, tool versions and the measured executable digest.
It is 371,316 bytes, so the raw observations are kept rather than discarded.

Each size has cold parse and cold check, body-edit parse/check, signature-edit
parse/check, syntax-break parse/refused check, and completion-point parsing.
W additionally has dependent rechecking. Cold W covers the full workspace;
W edits cover its 400-line W00 module. Dependent timing covers checking 49
already parsed, newly pinned modules, not graph traversal or parsing them.

`cold` means new parser/checker work without a prior result cache in an already
running process. It does not measure cold OS caches or application startup.
Cold check receives an already parsed tree. Input loading, compilation, process
startup, JSON transport, reporting and preflight comparison are outside the
function timers. Result allocation/disposal and the parser's source hash are
inside. Full IR equality is checked before measurement; individual timed samples
check the expected outcome count. Median averages the middle pair; p95/p99 use
nearest rank. Smoke samples are excluded from those statistics.

The **15,000 function samples are not RTE.52's 10,800 input-to-frame samples**.
No bar, route or editor path gets a pass/fail result. No incremental architecture,
completion list, activation policy, navigation UI or general language checker
has been implemented or selected. This finite corpus proves no equivalence
outside its checked inputs or full LG1-LG4/EV1 conformance. Semantic checking
retains LX-8's first-refusal behavior, not a multi-error semantic service.

The card's **Cannot show** remains: latency of arbitrary user views,
learnability, design quality, or a pass for another candidate/build. This run
also cannot show end-to-end input/feedback latency, both-puzzle behavior,
renderer gates, charter E2, long-session behavior, real mechanical correctness,
or a syntax/language recommendation. A reached wrapper limit is no pass; this
slice completed within the allowed wrapper time.

## Commands and remaining handoff

Commands actually run from the repository root:

```text
python --version
rustc --version
rustc -vV
python work/experiments/language-lx/lx4-editor/generate.py --freeze
python work/experiments/language-lx/lx4-editor/check.py
python -u work/experiments/language-lx/lx4-editor/measure.py --record
```

The acceptance command is run twice for final packet handoff. It compares
frozen source/input digests, regenerates modules under `build/`, replays edits,
checks both front ends, deletes build products even on failure, and confirms the
remaining folder is byte-identical. It compares no timing values, rewrites no
report, and refreshes no digest. Its final console results accompany this packet.

No required headless implementation item remains unfinished. Independent review
and integration remain with the calling integrator; no nested wrapper review,
commit, staging or Git ref change occurred in this offline implementation call.

The following are **not run here**, with commands from the relevant checkout root:

| Remaining run | Exact command / prerequisite |
| --- | --- |
| Recheck the integrated candidate | `python work/experiments/language-lx/lx4-editor/check.py` |
| Quiet owner-machine headless remeasurement outside this sandbox; still a function lead | `python work/experiments/language-lx/lx4-editor/measure.py --record` |
| WSL/Linux source acceptance, with local Python and Rust available | `python3 work/experiments/language-lx/lx4-editor/check.py` |
| WSL/Linux headless lead in its separate checkout | `python3 work/experiments/language-lx/lx4-editor/measure.py --record` |
| Replace the stand-in after the three LP3 tool versions freeze | First adapt the generator's seed provider to those frozen tools, then `python work/experiments/language-lx/lx4-editor/generate.py --freeze`, `python work/experiments/language-lx/lx4-editor/check.py`, and `python work/experiments/language-lx/lx4-editor/measure.py --record` |

`--record` deliberately replaces observation files for that new run; preserve
this frozen record before using it for another host/candidate. It is never used
by acceptance. New source/input identities require new measurements and a new
report, not relabelling these observations.

The rest of RTE.51 and all decisive timing are **not run here**: both puzzles'
fixture end states, D6 conditions, a pinned PresentMon/D7 capture, owned-window
message posting and frame-ID correlations, automatic rendering, W3 capture,
at least 100 scripted turns per series for E2, and the full 10,800-sample series.
The editor/renderer candidates, puzzle fixtures and owner-agreed foreground
window do not exist in this slice. **There is no runnable command for that full
protocol yet; none is invented here.** Its harness and allowance must be supplied
before running it. No frame, renderer-gate or RTE.53 decision is claimed.

Decision: **open**; decider: **owner**.
