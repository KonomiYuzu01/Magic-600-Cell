# ADR-L03: exactness, the numeric tower and the algebras

Status: **Accepted** (owner, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). The owner accepted state-contract amendment A6 separately
on the same day, after EN1's Astra check, so A6 is in force where this record calls it proposed.

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
  the A2 filter with its proven bound and in the shell's gesture producer. Under proposed state-contract amendment A6,
  the kernel receives exact request bytes, and menu drags compare candidates exactly in Q(√5)
  (owner, 6 November 2026, decision 8). KN3.4, KN3.7 to KN3.9, DET.31 and DET.33 describe that direction. A6 is not in
  force until EN1's Astra check and the owner.
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

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

Open mechanisms:

- How the type system stops a comparison of approximate values from steering an exact decision (LG2.5). LP1 settles it.
- The final wire encoding of new big integers and rationals. PS1.5 and FP.6 draft it, and it settles with the migration
  format freeze.
- Which numeric libraries implement the tower, and at what cost. LX-5 measures them, and a copyleft dependency needs the
  owner (RTE.54, RTE.55).
- Acceptance of proposed state-contract amendment A6 through EN1's Astra check and then the owner (KN3.4, RTE.65).
  Decision 8 chose exact requests, and the owner chose the free-mode grid (KN3.7). The shell's gesture producer remains
  an integrator default the owner may overrule (KN3.9).

This record answers no owner question, and it presets none of the routes. Decision 8 has since chosen the exact-request
direction; accepting this ADR would not accept A6.

## Why

- The evidence already shows the risk at the one float site the contract lists. In a local Windows CPython check, cosine
  and sine behind the A1 map were not correctly rounded for 484 and 436 of 13,600 requests, so another maths library can
  return the neighbouring value (DT-L1 R2-E2).
- Exact display raises an unmeasured performance concern. RPI.31 counts 18,288,000 vertex evaluations per frame at
  full detail; this is a workload count, not a throughput measurement or proof that exact display misses the frame-rate
  gate. Approximate display remains the proposed choice, with an explicit typed crossing and no exact decisions.
- The tower is small because the mathematics is small. Five kinds cover what the state contract and the research use.
  Adding host numeric types would invite the wraparound DET.13 rules out.
- Keeping the algebra above the polytope kernel in the library lets readers see its source (INV-12), and each decision
  the kernel keeps has an oracle agreement test (KN2.4).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Tolerance-based decisions with floats | A tolerance gives no proof at a boundary, and INV-4 forbids it |
| Exact evaluation of display too | Performance is unmeasured; RPI.31 counts work only. Approximate display is proposed because display makes no exact decisions; exact rendering's frame-rate feasibility remains unverified |
| The host language's numeric tower | Hosts differ on overflow, on rationals and on Q(√5), which none has; every route would still need KN3 (RTE.5) |
| All algebra inside the kernel | Larger trusted code, and less of it can be read and taught as source |

## Consequences

- Every library and tool pays the cost of exact arithmetic. Its size can grow with operands, which KN3.6 makes visible
  in the budget. How much that costs is unknown until LX-5.
- A tool author has to write the conversion to floats by hand and name its precision. That is the intended friction.
- A result computed through a float cannot be fed back into a decision. A program that wants the exact value has to
  compute it exactly.
- LX-9 found cross-runtime differences. Proposed amendment A6 uses exact dyadic Cayley, half-turn or bound menu-letter
  requests and exact menu comparison (owner, 6 November 2026, decision 8). KN3.7 and KN3.8 refuse malformed,
  noncanonical or oversize bytes without repair.

## Evidence

Classes: source and synthetic. The rounding count above is synthetic evidence from one Windows CPython build. It is not
DirectX or performance evidence. LX-9's later comparisons and exact-request follow-up are recorded in
`work/experiments/language-lx/lx9-input-map/RESULT.md` and
`work/experiments/language-lx/lx9-exact-request/RESULT.md`.
They supply synthetic evidence for the amendment direction, with their recorded limits. The B11 figure of 6,775 digits
and the claim that no table on record exceeds three decimal digits are source observations (KN3.2). LX-5's first-pass
source and synthetic results are in `work/experiments/language-lx/lx5-arithmetic/RESULT.md`; they select no library
and establish no performance pass (RTE.82). The engine is written in Rust (owner, 14 November 2026, LQ8; RTE.17), and
LP2 chooses the big-integer implementation on LX-5's decisive data, whose run the owner accepted (RTE.82). The
conformance tests are requirements only.

## What the owner accepts

Accepting this record accepts ADR-L03.1 to ADR-L03.6 and the [D] clauses they cite. It accepts no library, and it
accepts no amendment to the state contract.

The owner may overrule this integrator choice: the line between kernel and library in ADR-L03.3.
