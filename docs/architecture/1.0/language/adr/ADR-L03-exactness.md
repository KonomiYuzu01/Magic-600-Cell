# ADR-L03: exactness, the numeric tower and the algebras

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: partition INV-4, KN3 and LG2; [chapter 01](../architecture/01-kernel.md) KN3; [chapter
02](../architecture/02-language-core.md) LG1, LG2; [chapter 09](../architecture/09-determinism.md) DET.13, DET.31 to
DET.34. Round table: DT-L1 M4, R2-E2.

## Context

The puzzle's mathematics lives in exact rings: poses and cut offsets in Q(√5), the invariant ν over F2 and F5, costs and
heights in unbounded integers. Screens live in floats. Under I_a entry heights are unbounded, so a pose entry can exceed
2^64 (KN3.2). A decision made in floats can be wrong in a way no test on a sample shows, and one made on a host's
machine integer can wrap.

A language has to make the exact path the easy one, and it has to make the crossing into floats visible.

## Decision

- **ADR-L03.1** Every decision is exact, and nothing rounds silently (INV-4, KN3.1). Floats appear in display plans, in
  the A2 filter with its proven bound and in the A1 input map, including menu-mode drags. Those are the only sites
  (KN3.4, KN3.5, DET.31, DET.33).
- **ADR-L03.2** The kernel provides five exact number kinds, each with stated operations and a cost by operand size:
  unbounded integers, rationals, Q(√5), F2 and F5, and exact matrices (KN3.2, KN3.6).
- **ADR-L03.3** Exact polytope operations live in the kernel. The small exact algebra above them, meaning quaternions,
  the icosian order and its tree, and rows over F2 and F5, is library code (KN3.3, LB1).
- **ADR-L03.4** Exact and approximate values are separate types with separate literals. Exact-to-approximate conversion
  is explicit and names a precision, and there is no conversion back (LG1.6, LG1.7, LG2.4).
- **ADR-L03.5** Costs are exact nonnegative integers indexed by puzzle and unit (LG2.6). A word's exact length is an
  unbounded integer that names its alphabet (LG2.16). Host word size cannot cause wraparound or silent narrowing
  (DET.13).
- **ADR-L03.6** A stored Q(√5) value is the integer triple `[a,b,d]` for (a+b√5)/d, with positive d and gcd 1 (PS1.4).
  Decision fields hold exact values, never floats.

## What this record does not decide

Open whatever the owner answers:

- How the type system stops a comparison of approximate values from steering an exact decision (LG2.5). LP1 settles it.
- The wire encoding of new big integers and rationals (PS1.5).
- Which numeric libraries implement the tower, and at what cost. LX-5 measures them, and a copyleft dependency needs the
  owner (RTE.54, RTE.55).
- Whether fresh input-map choices agree across runtimes (DET.34). LX-9 decides, and a difference amends the state
  contract through EN1's Astra check and then the owner (KN3.4, RTE.65).

This record answers no owner question, and it presets none of the routes.

## Why

- The evidence already shows the risk at the one float site the contract lists. In a local Windows CPython check, cosine
  and sine behind the A1 map were not correctly rounded for 484 and 436 of 13,600 requests, so another maths library can
  return the neighbouring value (DT-L1 R2-E2).
- Display cannot be exact at frame rate. A chart in the projection chain costs 18,288,000 vertex evaluations per frame
  at full detail (RPI.31). So the boundary has to be a typed crossing, not a wish that floats go away.
- The tower is small because the mathematics is small. Five kinds cover what the state contract and the research use.
  Adding host numeric types would invite the wraparound DET.13 rules out.
- Keeping the algebra above the polytope kernel in the library lets readers see its source (INV-12), and each decision
  the kernel keeps has an oracle agreement test (KN2.4).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Tolerance-based decisions with floats | A tolerance gives no proof at a boundary, and INV-4 forbids it |
| Exact evaluation of display too | Not affordable at frame rate (RPI.31), and nothing needs it: display decides nothing |
| The host language's numeric tower | Hosts differ on overflow, on rationals and on Q(√5), which none has; every route would still need KN3 (RTE.5) |
| All algebra inside the kernel | Larger trusted code, and less of it can be read and taught as source |

## Consequences

- Every library and tool pays the cost of exact arithmetic. Its size can grow with operands, which KN3.6 makes visible
  in the budget. How much that costs is unknown until LX-5.
- A tool author has to write the conversion to floats by hand and name its precision. That is the intended friction.
- A result computed through a float cannot be fed back into a decision. A program that wants the exact value has to
  compute it exactly.
- If LX-9 finds a difference, the state contract changes: the request becomes a dyadic Cayley vector read exactly, and a
  drag compares its 60 candidates exactly in Q(√5).

## Evidence

Classes: source and synthetic. The rounding count above is synthetic evidence from one Windows CPython build. It is not
DirectX or performance evidence, and no other runtime has been compared. The B11 figure of 6,775 digits and the claim
that no table on record exceeds three decimal digits are source observations (KN3.2). LX-5 and LX-9 have not run, and
the conformance tests are requirements only.

## What the owner accepts

Accepting this record accepts ADR-L03.1 to ADR-L03.6 and the [D] clauses they cite. It accepts no library, and it
accepts no amendment to the state contract.

The owner may overrule this integrator choice: the line between kernel and library in ADR-L03.3.
