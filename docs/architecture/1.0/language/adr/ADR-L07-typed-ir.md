# ADR-L07: the typed IR as the stable contract

Status: **Proposed.** Not accepted. **Held for LP1: it follows the route choice** (partition section 17, PHS.10). The
owner accepts or rejects it then ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the owner").

Rests on: partition LG4 and section 12; [chapter 02](../architecture/02-language-core.md) LG1, LG4; [chapter
11](../architecture/11-route-and-spikes.md) sections 1 to 5. Round table: DT-L1 E4.

## Context

A language outlives its first implementation. Solvers will save tools, macros and run records, and each of those has to
mean the same thing years later, whatever runs it. The keyboard, the GUI and source text should also agree on what an
operation means.

Something has to be the contract for that meaning. Three things could be: the source text, the executor's own bytecode,
or a typed intermediate form the project owns. Under routes RT1 to RT3 the project already has the third.

## Decision

Under a route that has it, which means RT1, RT2 lowered to it, or RT3:

- **ADR-L07.1** The typed IR is the versioned execution contract shared by source, GUI-originated operations and keys.
  It preserves types, effect requirements, literal word order, source associations and the meaning of resource charges.
  Sharing the contract does not route every key through the interpreter (LG4.1, LG4.13).
- **ADR-L07.2** Lowering produces only operations that check against the stated families. No family provides a raw store
  write, a replacement of the authoritative state, a commit token, a renderer call, or a process or network call
  (LG4.2).
- **ADR-L07.3** Editions and IR versions are separate. An IR change that alters saved meaning or budget stopping points
  needs a recorded version change, and an unsupported version is refused (LG1.1, LG1.2, LG4.1, PS2.1).
- **ADR-L07.4** Evaluation is deterministic with one thread per run. Observable order never depends on addresses,
  hash-table layout or worker completion (LG4.3).
- **ADR-L07.5** The starting executor is an interpreter. No machine-code backend, own VM or JIT comes before measurement
  supports it (LG4.4).
- **ADR-L07.6** Whole-state operations lower to kernel batches, and compressed words stream to the kernel in chunks
  (LG4.5, LG4.6).
- **ADR-L07.7** Evaluation enforces the deterministic budget throughout, and cancellation is cooperative (LG4.7, LG4.9).
- **ADR-L07.8** IR operations retain their source module digest and location, so debugging and tracing can reach the
  source that ran (LG4.12).

## What this record does not decide

Open whatever the owner answers:

- The instruction set. LG4.2 fixes families, not instructions. LP1's specification fixes the operations.
- The step granularity and charge schedule (LG4.8, DET.20).

Conditional on an owner question:

- **LQ9:** the route. RT4 has no such IR by definition. It would either lower its approved subset into the common IR, or
  specify and verify an equivalent contract with its own compatibility and metering evidence (RTE.6, LG4.14). This
  record takes no position between those.
- **LQ10:** under its alternative of freezing syntax too, source text carries part of the contract.
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
| Source text as the contract | Every parser or checker change becomes a saved-meaning change. It stays open only as LQ10's alternative |
| No stable contract, with each release re-reading source | Budget stopping points and saved meaning could change silently, against PS2.2 |
| A machine-code backend from the start | No measurement supports it (LG4.4) |

## Consequences

- The project owns one more versioned artefact, its migrations and its tests.
- An IR does not make the front end cheap to change. Source, diagnostics and module contracts become saved-work
  dependencies, so the front end stays the hard choice to reverse under RT1 and RT3 (RTE.9, RTE.14), and a borrowed VM
  does not remove that commitment (RTE.12).
- The interpreter may turn out too slow for some tools. LX-1 reports, and until then no number supports either view.
- GUI operations have to lower to the same forms, so a button cannot do something a program could not express.

## Evidence

Class: source. No IR, checker or interpreter exists. CT-LG4 tests are requirements only. LX-1 to LX-5 are the inputs to
this decision, and none has run.

## What the owner accepts

At LP1, with the route, accepting this record accepts ADR-L07.1 to ADR-L07.8 under the route chosen, or the replacement
contract RT4 would need. Nothing here is accepted at LP0.
