# E-2.4-L8 preparation result

LX card: LX-8, chapter 11 RTE.62 and RTE.63. Experiment ID: E-2.4-L8.
Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2
and the recorded LQ18 reading. Decision: **open**; decider: **owner**.
Time box: **what can be done now; one wrapper call of at most two hours**.
A reached limit is no pass.

Evidence class: **source**. Human evidence: **not run**. Source preparation
acceptance: **passed**, exit 0 on 5 November 2026. This is not a human LX-8 pass
or CT-RTE-18 result. No candidate is ranked or recommended.

## Frozen scope

Repository base checked locally: `1e7d0074`, the packet's `claude/language-spikes`
baseline. No commit or other Git mutation was made. The briefing digest matched
the packet: `d7412a7b77a79c57836909503e39ce6004ed04b90c256114b1d3f52b700a381c`.

| Frozen item | SHA-256 |
| --- | --- |
| Candidate sources and descriptions aggregate | `c8b8e55909b8a27c63b52989efb03823639506721268d0fd75890a84a309b9fc` |
| Source, expected-IR, interface and recovery inputs aggregate | `5887badcf5476e66b34ce28a9ee40ecacbc11e5d761b9a09a6e38718c2551436` |
| `fixtures/MANIFEST.json` | `bb75ba9090186b63e2c6cfa7f0014e7144912901cb23a5e9f805aef79b3e42fb` |

All paths in this report below the experiment are relative to
`work/experiments/language-lx/lx8-surfaces/`. The manifest enumerates individual
SHA-256 values. Its aggregates hash UTF-8 JSON path-to-digest maps with sorted
keys and compact separators. The candidate map includes the shared lexer,
catalogue, lowerer, all three parsers, checker, independent corpus builder,
candidate descriptions, `ir.md`, `CARD.md` and `PROTOCOL-NEEDS.md`. The input map
contains the 51 source files, 17 expected JSON files, one shared interface and
48 recovery files. This report is an observation after the freeze, not a
self-hashed input. There is no compiled build.

## Commands and tool versions

Only installed CPython 3.15.1 and its standard library were used. `python
--version` reported `Python 3.15.1`. No package, compiler, network, kernel,
renderer, database or personal session was used.

| Command from repository root | Observation |
| --- | --- |
| `python work/experiments/language-lx/lx8-surfaces/fixtures/build.py` | Ran here, exit 0; produced the frozen candidate/input identities above |
| `python work/experiments/language-lx/lx8-surfaces/check.py` | Ran here, exit 0; output below |

The builder authors the semantic oracle independently of the implementation
and generates the three source spellings. The check reads those frozen inputs;
it never regenerates expected outputs. Acceptance writes no files and creates
no bytecode cache, build directory or binary, on success or failure. All 133
persisted files are UTF-8/LF text, with broken byte inputs encoded as hex JSON.

## Actual result

| Check | Observed result |
| --- | --- |
| Valid corpus | 17 programs per candidate, 51 parses and lowerings; every result equals its independent expected JSON and the other two candidates |
| LG2.12 type surface | Eight families covered, with all listed constructors represented except the forbidden shell-only user-action token |
| Exact values | A 5,001-digit integer, normalized rationals and Q(sqrt(5)) coefficient pairs preserved without a host float |
| Approximation | Separate approximate kinds at 24, 53 and 113 declared bits; explicit exact-to-approximate operation retained, no rounding executed |
| Words | Retained and fixture-menu literal order, inverse/identity letters, equality-key type separation and compressed structure retained; exact length `3 * 10^80 + 4` without expansion |
| Effects and definitions | Effect-bearing and forward-calling helpers, a pinned module interface, workspace/preview command definitions, a revision-input view, explicit file/random grants and budget requirements preserved |
| Broken drafts | 16 per candidate, 48 total; 14 textual syntax drafts and two byte-contract cases per candidate; all six `LG1:syntax.*` codes observed |
| Recovery controls | Repeated parse results stable; Unicode comment insertion shifts locations correctly without changing codes; changed diagnostic wording does not permit lowering; two independent errors retain the later valid declaration |
| Semantic mutation controls | 45 passed: 12 valid semantic changes remained observable and 33 invalid type/effect/authority edits were refused |
| Artifact checks | Frozen digests match; no unexpected generated artifacts; all stored text satisfies UTF-8/LF rules |

Acceptance's summary was:

```text
PASS: 17 programs x 3 candidates = 51 parses and golden/cross-candidate IR comparisons.
PASS: 48 broken drafts ({'candidate-a': 16, 'candidate-b': 16, 'candidate-c': 16}); all 6 LG1:syntax.* codes; recovery locations and non-lowering.
PASS: 8 domain families; exact literals; effects; word order/length; 45 semantic mutation controls.
Evidence: source only. Human run: not run. Decision: open; decider: owner.
```

Expected negative results include absent/unsupported editions, invalid UTF-8 and
CRLF, zero denominators, unsupported approximate precision and nonfinite text,
unmarked decimal fractions, malformed Q(sqrt(5)) coefficients, zero generators,
missing expressions/delimiters, bad strings, parameter syntax and block layout.
No broken draft lowered. The negative semantic controls reject mismatched
models, modes and cost units, erased effects, missing module/command authority,
incomplete verdict cases, equality keys used as words, workspace-capable views,
unavailable dependency identities and a language-visible user-action token.
No unexpected acceptance failure or semantic mismatch was observed.

## Meaning and limits

Equivalence covers the exact operations, indexed types, effects, ordered
operands and bindings, verdict arms and payloads, literal order, approximate
precision, compression lengths, dependency-interface identities, command
metadata and declared charges of these frozen programs. Only source associations
(input digest and location) are removed for comparison. No optimizer, rewrite or
execution is used to make different programs agree.

Family coverage is type-surface and representative-operation coverage. Opaque
state, claim, representation, outcome and research parameters are not real
kernel values. The shared imported interface is a signature fixture, not an
implemented dependency. Declared charges are synthetic numbers, not LG4 tariffs,
measured work, budget enforcement or stopping-point evidence. No fixture proves
mechanical correctness, protection, admissibility, reachability, projection,
boundary-policy enforcement, rendering or a solving result.

Kill criteria remain those in `CARD.md`: no human run without the owner's
protocol; any semantic mismatch prevents comparison as equivalent candidates;
other human early-stop rules await the owner. A reached wrapper limit is no
pass; this preparation completed without reaching it.

The card's **Cannot show** remains: general learnability, product design
sign-off, or a syntax decision by authors' or models' agreement. The run also
does not show general equivalence outside this finite corpus, full LG1-LG4
conformance, formatter behavior, runtime semantics, Windows/DirectX behavior or
performance. There was no human run, ranking, preference measurement, interpreter,
real kernel call, WSL leg, GPU run or decisive timing. None is needed for this
packet's source acceptance. Open language mechanisms and all LQ decisions stay
open for the owner.

## Remaining handoff

No required source-preparation item remains unfinished. Independent integration
review remains with the integrator; no nested wrapper review ran inside this
offline, folder-scoped implementation packet.

After review and application to the integration candidate, the required recheck
on that candidate is **not run here**:

```text
python work/experiments/language-lx/lx8-surfaces/check.py
```

There are no additional native, GPU, Linux or timing commands required for this
LX-8 preparation. The owner must still set the human protocol fields in
`PROTOCOL-NEEDS.md`; a human-test command does not exist until that protocol and
its procedure are supplied. No such command or result is fabricated here.

Decision: **open**; decider: **owner**. No route, engine language, syntax,
language name, tariff, ADR or LQ answer is selected.
