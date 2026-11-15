# LX-8: equivalent syntax candidates

Experiment ID: **E-2.4-L8**. Decision: **open**; decider: **owner**.
Experiment code and frozen source fixtures only; no route, engine, syntax,
language name, tariff or LQ answer is selected.

The unchanged LX-8 card is chapter 11, RTE.62, at
`docs/architecture/1.0/language/architecture/11-route-and-spikes.md`:

| Field | Card |
| --- | --- |
| Question | Which of two or three syntax candidates over one typed IR can non-authors learn and use under the owner's protocol? |
| Decision it feeds | LQ13 before LP2 builds the parser; working name LQ17 remains the owner's choice |
| Hypothesis | None supplied; author or model preferences are not human evidence |
| Method | Prepare equivalent typed operations, effects, literal order and charges under LG1.8; participants, tasks, observations and comparison are set only by the owner's protocol |
| Time box | Owner sets it with the protocol before a run |
| Kill criteria | No human run before that protocol exists; semantic mismatch prevents comparison as equivalent candidates; other early-stop criteria await the owner |
| Evidence class | human; equivalence of the semantic fixtures is separate source evidence |
| Cannot show | General learnability, product design sign-off, or a syntax decision by authors' or models' agreement |
| Product left behind | Equivalent candidate descriptions and source/IR fixtures; if allowed, the protocol's observations and owner decision |

RTE.63 leaves participant selection, tasks, sample plan, evaluation, stopping
rules and inconclusive results to the owner. There is no human protocol here.
Choosing now could remove the human test prerequisite but would not remove
LG1's semantic, recovery and edition obligations. No ranking is supplied.

## This allowed preparation

Owner allowance: `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2
and its LQ18 reading; experiment index `work/experiments/language-lx/README.md`.
Time box: **what can be done now; one wrapper call of at most two hours**.
A reached limit is no pass. The source-only preparation does not assign a time
box to the human test.

Three candidates (`candidate-a`, `candidate-b`, `candidate-c`) parse and lower
17 frozen programs each to `lx8-ir-0`. The expected JSON is authored independently
of the lowerer. Each has 14 textual broken drafts and two byte-contract cases
encoded as hex in JSON. Every semantic fixture is compared to its expected IR
and to the other surfaces. Only source associations differ. Altered source
locations and diagnostic text are controls, not human trials.

Acceptance command, from repository root:

```text
python work/experiments/language-lx/lx8-surfaces/check.py
```

Production command for deliberately regenerating/freeze-identifying the corpus:

```text
python work/experiments/language-lx/lx8-surfaces/fixtures/build.py
```

The latter creates a new candidate/input identity when source changes; it must
not be used to relabel an old result. Acceptance is read-only and creates no
bytecode cache, build directory or binary, including on failure. Every persisted
file remains UTF-8/LF inside this folder. No user data, network or installation
is used. The manifest freezes parser/lowerer/checker/printer and descriptive
sources, plus all source and expected-IR inputs. `RESULT.md` records observations
after the freeze and is deliberately outside the aggregate to avoid a self-hash.

Immediate preparation kill criteria: any parse, expected-IR, cross-candidate,
digest, type/effect, literal-order, charge or recovery check failure stops a
claim of equivalent candidates. No current source result licenses a human run.
If work is incomplete or the wrapper limit is reached, record the unfinished
items instead of a pass.

Domain references: `docs/progress/1.0/solving-workflow.md`,
`docs/progress/1.0/command-table.md`, and `research/jumbling/state-contract.md`.
They supply vocabulary and constraints, not executable kernels or selected UI.
The exact preparation scope and limits are in `ir.md` and `PROTOCOL-NEEDS.md`.
