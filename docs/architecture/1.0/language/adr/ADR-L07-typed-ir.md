# ADR-L07: the typed IR as the stable contract

Status: **Proposed.** Not accepted. It follows the route choice (partition section 17, PHS.10), and the owner chose
RT1 on 14 November 2026 (LQ9, RTE.21), so the typed IR applies. The owner accepts or rejects it after both LP1 rulings
([`AGENTS.md`](../../../../../AGENTS.md), "Ask the owner").

Rests on: partition LG4 and section 12; [chapter 02](../architecture/02-language-core.md) LG1, LG4; [chapter
11](../architecture/11-route-and-spikes.md) sections 1 to 5. Round table: DT-L1 E4.

## Context

A language outlives its first implementation. Solvers will save tools, macros and run records, and each of those has to
mean the same thing years later, whatever runs it. The keyboard, the GUI and source text should also agree on what an
operation means.

Something has to be the contract for that meaning. Three things could be: the source text, the executor's own bytecode,
or a typed intermediate form the project owns. Under routes RT1 to RT3 the project already has the third, and the owner
chose RT1 (RTE.21).

## Decision

Under a route that has it, which means RT1, RT2 lowered to it, or RT3. The owner chose RT1, whose interpreter runs in a
process of its own (owner, 14 November 2026, LQ9; RTE.21):

- **ADR-L07.1** The typed IR is the versioned execution contract shared by source, GUI-originated operations and keys.
  It preserves types, effect requirements, literal word order, source associations and the meaning of resource charges.
  Sharing the contract does not route every key through the interpreter (LG4.1, LG4.13).
- **ADR-L07.2** Lowering produces only operations that check against the stated families. No family provides a raw store
  write, a replacement of the authoritative state, a commit token, a renderer call, or a process or network call
  (LG4.2).
- **ADR-L07.3** Editions and IR versions are separate. An IR change that alters saved meaning or budget stopping points
  needs a recorded version change, and an unsupported version is refused (LG1.1, LG1.2, LG4.1, PS2.1).
  The migration format freeze covers only the language-neutral data core; syntax and IR freeze later, as PS2.6 and
  PS2.10 describe (owner, 6 November 2026, LQ10).
- **ADR-L07.4** Evaluation is deterministic with one thread per run. Observable order never depends on addresses,
  hash-table layout or worker completion (LG4.3).
- **ADR-L07.5** The starting executor is an interpreter. No machine-code backend, own VM or JIT comes before measurement
  supports it (LG4.4).
- **ADR-L07.6** Whole-state operations lower to kernel batches, and compressed words stream to the kernel in chunks
  (LG4.5, LG4.6).
- **ADR-L07.7** Evaluation enforces the deterministic budget throughout, and cancellation is cooperative (LG4.7,
  LG4.9). LG4.8 and DET.54 to DET.64 now describe `lp1-exec-draft-1`: semantic operations, admission boundaries,
  logical resources and continuation charges. This is a draft execution contract, not serialized IR or an accepted route.
- **ADR-L07.8** IR operations retain their source module digest and location, so debugging and tracing can reach the
  source that ran (LG4.12). EV1.10 supplies the source ranges and EV2.12 the semantic stepping protocol; a route
  still owes its executor-to-contract mapping, including suspended kernel calls.

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

Open mechanisms:

- Review and implementation of DET.54/DET.55's semantic operations, which now refine LG4.2's families. Serialized
  instruction encoding and each executor mapping remain to be supplied; the semantic draft is not an IR byte format.
- Review and conformance of LG4.8's proposed uniform steps with input-size tariffs, now detailed by DET.54 to
  DET.64. The fixed-weight alternative remains recorded and needs a different execution identity and compatibility
  treatment. Production tariffs and metering evidence are still owed; no alternative is selected by this record.

Answered owner question:

- **LQ9:** RT1, an own interpreter over this IR, in a process of its own (owner, 14 November 2026; RTE.21). RT4 has no
  such IR by definition; its lowering or replacement contract (RTE.6, LG4.14) records the unselected alternative.

Waiting owner question:

- **LQ13:** the syntax. Any candidate lowers to the same IR (LG1.8).

## Why

- Every route needs saved meaning to survive a change of executor. A route change must preserve the language-neutral
  data core and the meaning of saved work (RTE.3). An owned IR is the piece that makes that possible below the front
  end.
- Domain obligations live at this level: literal order, exactness, effects, charges (RTE.4). A borrowed bytecode knows
  none of them.
- Measurement comes before any faster executor, because nothing yet says the interpreter is too slow, and each extra
  executor would need the same conformance proof.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| The host VM's bytecode as the contract | Ties saved meaning to a third party's versions, and it has no place for types, effects or charges |
| Source text as the contract | Rejected alternative of LQ10 (owner, 6 November 2026). Every parser or checker change becomes a saved-meaning change |
| No stable contract, with each release re-reading source | Budget stopping points and saved meaning could change silently, against PS2.2 |
| A machine-code backend from the start | No measurement supports it (LG4.4) |

## Consequences

- The project owns one more versioned artefact, its migrations and its tests.
- An IR does not make the front end cheap to change. Source, diagnostics and module contracts become saved-work
  dependencies, so the front end stays the hard choice to reverse under RT1 and RT3 (RTE.9, RTE.14), and a borrowed VM
  does not remove that commitment (RTE.12).
- The interpreter may turn out too slow for some tools. LX-1's first-pass leads do not establish product cost; its
  decisive run is an LP3 acceptance check, and a failure reopens LQ9 (RTE.21, RTE.82).
- GUI operations have to lower to the same forms, so a button cannot do something a program could not express.

## Evidence

Class: source. No product IR, checker or interpreter is established by this record. CT-LG4 and CT-DET-25 to
CT-DET-32 remain requirements. The abstract fixtures in `contract-vectors/` describe the draft meter; they do not
establish product conformance or an executor mapping. LX-1 to LX-5 have first-pass records under
`work/experiments/language-lx/`; their `RESULT.md` files keep each run's limits (RTE.82). The owner chose RT1 on
those leads (RTE.21); the decisive LX-1, LX-2 and LX-4 runs are LP3 acceptance checks.

## What the owner accepts

After both LP1 rulings, accepting this record accepts ADR-L07.1 to ADR-L07.8 under RT1, the route the owner chose
(RTE.21). Nothing here is accepted before them.
