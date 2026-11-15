# ADR-L06: effects, capabilities and flow control

Status: **Accepted** (owner, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). **The owner chose the form of flow control on 14 November 2026: capability separation** (LQ21, RTE.24).

Rests on: partition INV-3, INV-10, INV-11, LG3 and BM2; [chapter 02](../architecture/02-language-core.md) LG3; [chapter
07](../architecture/07-boundary.md) sections 4 to 6; [chapter 09](../architecture/09-determinism.md) section 5. Round
table: DT-L1 M5, R2-E3, R3-M3.

## Context

What a program can do should be visible before it runs, and it should come from an explicit grant rather than from the
machine around it. Two needs meet here. One is safety: a tool from someone else should not read a file it was not given.
The other is the boundary: the owner's principle concerns who chose a word, and the part of that question software can
check is a flow property, namely which information reached a preview (BND.12).

Two forms of flow control can do that job. They differ in cost, in what they refuse and in what a record can say. The
owner chose capability separation on LX-1's cost leads and LX-3's refusal counts (owner, 14 November 2026, LQ21;
RTE.24, RTE.82).

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
- **ADR-L06.9** The form is capability separation (owner, 14 November 2026, LQ21; LG3.13, BND.39, RTE.24). A program
  in the solving profile either reads state-derived data or prepares previews, never both, apart from the fixed
  declassifiers that LQ2's joint setting enables. A tool that needs both uses such a declassifier or is split into a
  reading tool and a previewing tool. LX-3's remaining checks are LP3 acceptance checks.

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

**Decided under LQ21:** capability separation (ADR-L06.9). The other form stays on record and binds nothing:

- **Flow labels** (LG3.12, BND.38): statically in the checker, or also at run time.

LX-1 and LX-3 fed that choice. The owner accepts this record with the route record after both LP1 rulings (RTE.24 to
RTE.27, LG3.14).

Open mechanisms:

- Whether an invoked command may replace a preview already on screen (LG3.7, KN5.12).

The stop and emission channels are closed by capability separation, the third of BND.51's fixes (owner,
14 November 2026, LQ21). The other two apply to flow labels; they stay on record and bind nothing.

Waiting for the joint setting:

- **LQ2:** which flows, author classes, display sinks and declassifiers the policy covers. LQ21 chose the mechanism,
  not those permissions (RTE.27). See [ADR-L09](ADR-L09-boundary-mechanisms.md).

## Why

- Effects in types show a tool's reach at the point of reading it, and the checker can refuse a call before anything
  runs.
- Inheritance through calls and through function arguments closes the obvious route around a declaration (LG3.2).
- Ambient authority would also break reproducibility: a run that can read the clock or a directory has an input its key
  does not name (DET.4, DET.10).
- On LX-3's RT1 stand-in, capability separation refused all 20 must-refuse cases and falsely refused 2 of 35 allowed
  tools; both label forms refused 16 of 20 and left MR13, MR14, MR19 and MR20 unrefused (RTE.24). The two false
  refusals have a remedy, a fixed declassifier or a split tool. Separation also needs no per-value labels in the
  executor (RTE.25).
- These first-pass counts and LX-1's cost leads do not settle a product's refusal rate (BND.80, RTE.39, RTE.82).
  LX-3's remaining checks are LP3 acceptance checks.
- Deciding it after LP2 would be worse: saved tools would start being refused.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| A commit effect, gated by a confirmation dialog | Against INV-3: only the shell's explicit user action commits |
| Run-time permission checks with no effect types | A tool fails only when it reaches the call, and nothing shows its reach before it runs |
| Trusting declarations without inference | A declaration would become a grant, against LG3.9 |
| Flow labels, static or also at run time | Not selected (owner, 14 November 2026, LQ21). On LX-3's RT1 stand-in both forms left four must-refuse cases unrefused, and run-time labels need the project to own the executor's values (RTE.4, RTE.24, RTE.25) |
| Waiting for decisive LX-1 and LX-3 runs before choosing | The owner first chose to wait (owner, 6 November 2026, LQ21), then chose on the first-pass evidence and made LX-3's remaining checks LP3 acceptance checks (owner, 14 November 2026, LQ21) |
| Leaving flow control out of 1.0 | Provenance could not say what a preview depended on, and BM2 could not be built later without refusing saved tools |

## Consequences

- Capability separation is conservative and refuses some harmless tools; LX-3's stand-in refused 2 of 35 (BND.40,
  BND.80, RTE.24).
- A solver's own tool cannot both read the state and preview without a fixed declassifier, and a record says only
  whether a program read the state.
- Flow labels, the unselected form, would have needed a checker over exactly the source that runs, and run-time labels
  the project's ownership of the executor's values (RTE.4, RTE.25).
- No form closes every channel. A state-dependent stop can select among pending previews, a person can retype a
  word, and a display sink is a policy question (BND.46 to BND.49, BND.77).
- Authors write nothing for effects in the common case, because they are inferred. They do meet refusals they have to
  understand, so diagnostics have to name the capability and the call chain.

## Evidence

Classes: source and synthetic. No product checker exists. LX-1 and LX-3 ran first-pass experiments under the allowance
(owner, 6 November 2026, LQ18). Their records are in `work/experiments/language-lx/lx1-cost/RESULT.md` and
`work/experiments/language-lx/lx3-sandbox-flow/`, including the corpus, candidate and recheck `RESULT.md` files. These
are leads with recorded failures and limits (RTE.82). The owner chose LQ9 and LQ21 on them (owner, 14 November 2026;
RTE.21, RTE.24); LX-3's remaining checks are LP3 acceptance checks, and a failing one goes to the owner.
The stop-channel figure of about 15.1 bits per run is arithmetic from the mathematician's card, not a measurement.

## What the owner accepts

After both LP1 rulings, accepting this record accepts ADR-L06.1 to ADR-L06.9 and the [D] clauses they cite, with the
form of flow control the owner chose under LQ21 (RTE.24).

The owner may overrule this integrator addition: LG3.6, previews only inside a command the solver invokes.
