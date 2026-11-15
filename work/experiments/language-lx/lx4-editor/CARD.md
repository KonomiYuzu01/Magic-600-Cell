# LX-4: headless front-end lead

Experiment ID: **E-2.4-L4**. Decision: **open**; decider: **owner**.
Time box: **what can be done now; one wrapper call of at most two hours**.
A reached limit is no pass. This is experiment code, never product code.
It selects no route, engine language or syntax and answers no LQ.

The unchanged card in chapter 11, RTE.47, asks whether a route's editor path
meets the proposed feedback bars while turns and rendering meet their gates.
It feeds LQ9 and later EV1 acceptance. Its hypothesis is that the source corpus
supports representative sizes; no route latency is predicted. The full method
uses RTE.48 sizes, RTE.49 bars and all nine steps of RTE.51 on the owner's machine.
The owner agrees the foreground window and time box before that full run.

The full card's kill criteria remain: an S/M bar miss or any renderer/E2 miss
fails as RTE.53 states; keep complete slow series; retain invalid correctness
runs and repeat them whole; timeout leaves an incomplete record. Its evidence
class is performance on the owner's machine; source corpus sizes are not
performance. Its **Cannot show** row is: latency of arbitrary user views,
learnability, design quality, or a pass for another candidate/build.

This packet runs only the authorized headless part. It leaves a deterministic
module generator, frozen digests, a seeded edit script, two front ends and
function-timer series. It leaves no frame/input correlation or bar judgement.
Its immediate kill criterion is any frozen-input, parser, IR, diagnostic,
edit-replay or checker disagreement: no timer runs before that is resolved.
Unexpected per-sample refusal counts invalidate a timer run; all series are
retained and must be repeated after the cause is fixed. No slow observation
alone stops a series or passes/fails a proposed bar.

## Sources read

- `docs/architecture/1.0/language/architecture/11-route-and-spikes.md`,
  RTE.35 to RTE.37 and RTE.47 to RTE.53.
- `docs/architecture/1.0/language/architecture/04-environment.md`, EV1.1 to EV1.9.
- `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2 and its LQ18
  allowance; `work/experiments/language-lx/README.md`.
- `docs/development-guide/AGENT_BRIEFING.md` and `AGENTS.md`.
- `work/experiments/language-lx/lx8-surfaces/`: `RESULT.md`, `CARD.md`, `ir.md`,
  `syntax.py`, `lower.py`, `semantics.py`, `check.py`, the three candidate
  parsers/descriptions and the frozen fixture corpus/builder.
- `work/experiments/language-lx/lx5-arithmetic/candidate.rs`: source of the
  copied integer and SHA-256 routines in `numeric.rs` (unused operations removed,
  leading plus accepted). This is reuse of repository experiment code, no crate.

Moving chapters/cards/decisions are named, not digest-pinned. `MANIFEST.json`
pins this experiment's sources and its read-only LX-8 inputs. `RESULT.md`,
`SERIES.md` and `timings.json` are observation records outside the source hash.

## Frozen method

**RTE.51 step 1 asks for the three LP3 tools. Their experiment versions are not
frozen. These modules are a stand-in made from LX-8's 17 fixture programs, and
the module set must be regenerated from the three tools when they freeze.**
No claim of representativeness beyond this synthetic construction is made.

`generate.py` uses seed 2026110547 and candidate-a source fragments. Token-based
renaming changes local declaration names and their references, preserving
strings and metadata. S/M/L are exactly 200/1,000/5,000 LF-terminated lines.
W is 50 modules of 400 lines each, 20,000 lines total. Whole fixture fragments
repeat; the remaining short tail is comment padding. `MANIFEST.json` records
bytes, lines, source digests, each fragment and the import graph. The large exact
literal is preserved, so bytes per line need not resemble the research census.
Only S is retained as `sample.lxa8`; other modules exist under `build/` briefly.

W models shared base utilities, domain helpers, workflows and tool entry points,
with digest-pinned imports of earlier modules and actual typed calls across each
edge. It is an acyclic synthetic graph, not a real LP3 graph. Every module exposes
one `exported() -> Int` signature; equal signatures may share a content digest.
Private fragment declarations retain the rest of the LX-8 semantic workload.
Changing W00's export to Text also changes its body to Text, so the edited module
is valid. Re-pinning direct imports and checking the full 49-module transitive
closure yields 17 expected direct-dependent type refusals. Other dependencies
retain their declared interfaces; this is conservative rechecking, no incremental
algorithm or draft activation policy.

`edit_script.py` uses seed 2026110551. `EDITS.json` pins 16 edits: one of each of
inside-body, exported-signature, completion-trigger and syntax-break at S/M/L/W00.
Each starts from the original bytes, records replacement preimages and result
digests, and refuses a stale baseline. Syntax-break recovery cannot lower;
the completion trigger leaves a broken tail and measures parsing only.

Candidate-a's braces/keywords have the simplest grammar to port here: no layout
handling or pipelines in the measured surface. **This is no preference about
syntax.** Python reuses LX-8 unchanged. Rust independently lexes, parses and
checks, using only its standard library and LX-8's frozen signature/type catalogue
as data. The b/c Rust grammar adapters exist to validate all 48 unmodified broken
drafts; they are not measured. Acceptance additionally checks all 51 valid source
spellings. Complete parsed trees, IR, diagnostics including messages and source
spans, and semantic refusal messages are compared on identical source bytes.
No source field is erased in cross-host comparison. LX-8's authored golden IR
comparison still uses its own source-association exclusion.

The timer boundary is the parser and/or checker function call, including result
allocation/disposal and the parser's input hash. Reading source/interface files,
compilation, process start, JSON transport, preflight equivalence and reporting
are excluded. Rust uses `Instant`; Python uses `perf_counter_ns` with normal GC.
Both receive source bytes in memory and a preloaded digest-verified interface map.
Cold parse creates a new tree; cold check uses an already parsed tree and a fresh
checker. No prior result cache is used; OS caches/processes are warm. This is not
application startup. Edited-module parse/check is a full reparse and recheck.
Dependent checking uses already parsed, newly pinned inputs; graph traversal and
reparsing are outside that timer. There is no incremental architecture.

Each host/size/function has one smoke sample, then three complete series of 100,
with median, nearest-rank p95/p99 and largest reported separately for each series.
Cold W covers the full workspace; W edits cover W00 only. Expected syntax/type
refusals are counted per sample after full IR equivalence in preflight. There
are no mechanical states or puzzle hashes in this source-only workload.

**This is a function timer, a lead only. RTE.49's bars run from a key message to
a presented frame; no figure here passes or fails a bar. The proposed bars are
printed beside the figures only as context and are not compared.**

Everything needing a frame, window, PresentMon, both puzzles' fixtures or the
10,800-sample series is **not run here**. No completion list is constructed,
no view is executed and no renderer, owned-window queue, E2 turn workload or
foreground capture is implemented. There is no command for that full protocol
until those candidates/fixtures and the owner's window are supplied.

## Commands (repository root)

```text
python work/experiments/language-lx/lx4-editor/generate.py --freeze
python work/experiments/language-lx/lx4-editor/generate.py
python work/experiments/language-lx/lx4-editor/check.py
python work/experiments/language-lx/lx4-editor/measure.py --record
```

`--freeze` deliberately creates a new input/candidate identity; it never runs
from acceptance. `generate.py` without it regenerates under `build/`, prints
the set digest and removes `build/`. `check.py` regenerates, compares and replays
without timing or refreshed files; it removes `build/` on success and failure
and confirms the remaining files' paths and bytes match their initial snapshot.
An existing `build/` is refused for inspection rather than silently deleted.
`measure.py --record` runs preflight acceptance and replaces the two observation
files only after scratch cleanup; without `--record` it prints the series only.
No personal data, network, installation, external service or product change is used.

Inside `lx4-editor/`, the compiler command run by these helpers is:

```text
rustc --edition=2021 -C opt-level=3 -C debuginfo=0 frontend.rs -o build/frontend.exe
```

`TMP` and `TEMP` point to `build/tmp` in the compiler process. On Linux the
output is `build/frontend`; no cross-platform evidence is inferred from Windows.
No generated executable, object, PDB, bytecode cache or large module is retained.
