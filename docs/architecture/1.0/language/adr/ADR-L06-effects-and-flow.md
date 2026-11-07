# ADR-L06: effects, capabilities and flow control

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner"). **The form of flow control is held for LP1** (PHS.10). The owner accepts ADRs after both LP1 rulings.

Rests on: partition INV-3, INV-10, INV-11, LG3 and BM2; [chapter 02](../architecture/02-language-core.md) LG3; [chapter
07](../architecture/07-boundary.md) sections 4 to 6; [chapter 09](../architecture/09-determinism.md) section 5. Round
table: DT-L1 M5, R2-E3, R3-M3.

## Context

What a program can do should be visible before it runs, and it should come from an explicit grant rather than from the
machine around it. Two needs meet here. One is safety: a tool from someone else should not read a file it was not given.
The other is the boundary: the owner's principle concerns who chose a word, and the part of that question software can
check is a flow property, namely which information reached a preview (BND.12).

Two forms of flow control can do that job. They differ in cost, in what they refuse and in what a record can say. The
decisive evidence to choose between them is still owed (RTE.82).

## Decision

- **ADR-L06.1** Function types carry effects. Eight effect families describe what a function can request, and none
  grants direct access to the engine, store or renderer (LG3.1). Effects and capability requirements are inferred,
  checked and inherited through calls, and an annotation cannot erase an inferred effect (LG3.2). Module boundaries
  preserve them (LG5.7).
- **ADR-L06.2** There is no program effect that commits. A program cannot mint, receive, persist or forward a token
  (LG3.4, KN5.15).
- **ADR-L06.3** Views have no workspace-write effect and no preview effect, including through callees. A workspace
  change and a preview occur only inside a command the solver invokes (LG3.5, LG3.6).
- **ADR-L06.4** There is no ambient authority. Files, clock and randomness are capabilities granted explicitly. Network
  and process access are not granted in 1.0 (LG3.8, DET.10, DET.42).
- **ADR-L06.5** A module declares what it needs, and the host grants it per profile and per run. A declaration requests
  authority and never supplies it (LG3.9, DET.37).
- **ADR-L06.6** Static success never replaces the engine's run-time revalidation (LG3.10).
- **ADR-L06.7** Provenance survives calls and persistence under either form of flow control (LG3.11, PS1.33).
- **ADR-L06.8** One form of flow control is chosen before LP2, because checks added after the checker and IR exist would
  refuse tools that had been allowed to save (BND.40, RTE.26). Choosing a form does not switch the flow rule on
  (BND.41). Neither form proves authorship or understanding (LG3.15), and neither is evidence that a claim is true
  (BND.45).

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

**Held for LP1, under LQ21:**

- **Flow labels** (LG3.12, BND.38): statically in the checker, or also at run time.
- **Capability separation** (LG3.13, BND.39): a program in the solving profile either reads state-derived data or
  prepares previews, never both, apart from the declassifiers.

LX-1 and LX-3 feed that choice, and the owner accepts the form with the route (RTE.24 to RTE.27, LG3.14).

Open mechanisms:

- Whether an invoked command may replace a preview already on screen (LG3.7, KN5.12).
- The three fixes for the stop and emission channels, none of them chosen (BND.51).

Waiting for the joint setting:

- **LQ2:** which flows, author classes, display sinks and declassifiers the policy covers. LQ21 decides the mechanism,
  not those permissions (RTE.27). See [ADR-L09](ADR-L09-boundary-mechanisms.md).

## Why

- Effects in types show a tool's reach at the point of reading it, and the checker can refuse a call before anything
  runs.
- Inheritance through calls and through function arguments closes the obvious route around a declaration (LG3.2).
- Ambient authority would also break reproducibility: a run that can read the clock or a directory has an input its key
  does not name (DET.4, DET.10).
- The first-pass LX-3 refusal counts and LX-1 cost leads do not settle a product's refusal rate or run-time label cost
  (BND.80, RTE.39, RTE.82). The decisive evidence is still owed.
- Deciding it after LP2 would be worse: saved tools would start being refused.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| A commit effect, gated by a confirmation dialog | Against INV-3: only the shell's explicit user action commits |
| Run-time permission checks with no effect types | A tool fails only when it reaches the call, and nothing shows its reach before it runs |
| Trusting declarations without inference | A declaration would become a grant, against LG3.9 |
| Choosing a form now | The first-pass refusal and cost leads do not supply the decisive evidence (RTE.82) |
| Leaving flow control out of 1.0 | Provenance could not say what a preview depended on, and BM2 could not be built later without refusing saved tools |

## Consequences

- Either form is conservative and refuses some harmless tools (BND.40, BND.80).
- Under capability separation a solver's own tool cannot both read the state and preview, and a record says only whether
  a program read the state.
- Under flow labels the route needs a checker over exactly the source that runs, and run-time labels need the project to
  own the executor's values (RTE.4, RTE.25).
- Neither form closes every channel. A state-dependent stop can select among pending previews, a person can retype a
  word, and a display sink is a policy question (BND.46 to BND.49, BND.77).
- Authors write nothing for effects in the common case, because they are inferred. They do meet refusals they have to
  understand, so diagnostics have to name the capability and the call chain.

## Evidence

Classes: source and synthetic. No product checker exists. LX-1 and LX-3 ran first-pass experiments under the allowance
(owner, 6 November 2026, LQ18). Their records are in `work/experiments/language-lx/lx1-cost/RESULT.md` and
`work/experiments/language-lx/lx3-sandbox-flow/`, including the corpus, candidate and recheck `RESULT.md` files. These
are leads with recorded failures and limits; they decide neither LQ9 nor LQ21 (RTE.82). Decisive runs wait for the
owner's window (owner, 6 November 2026, decision 15).
The stop-channel figure of about 15.1 bits per run is arithmetic from the mathematician's card, not a measurement.

## What the owner accepts

After both LP1 rulings, accepting this record accepts ADR-L06.1 to ADR-L06.8 and the [D] clauses they cite, with the
form of flow control settled by LQ21 on LP1's evidence.

The owner may overrule this integrator addition: LG3.6, previews only inside a command the solver invokes.
