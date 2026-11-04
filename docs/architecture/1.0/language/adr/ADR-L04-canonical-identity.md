# ADR-L04: canonical identity, the identity tower and source maps

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: partition INV-6, INV-7; [chapter 01](../architecture/01-kernel.md) KN1; [chapter
02](../architecture/02-language-core.md) LG2; [chapter 10](../architecture/10-renderer-platform.md) RPI.20 to RPI.26.
Round table: DT-L1 M3, R2-E2, note N1, R3-M2; explainer P-M8.

## Context

"The same piece", "the same state" and "the same position" mean different things at different levels of the puzzle. Two
jumbled states can show the same stickers and differ in pose. Two states can have equal cosets and different shapes. A
tool that compares at the wrong level gives an answer that looks right and is wrong, and a claim that crosses levels
without a rule proves nothing.

The language needs names for these levels, exact maps between them, and a type system that keeps them apart. Drawing
adds a further risk: what is on screen is an instance, and it must never become an identity.

## Decision

- **ADR-L04.1** Objects are addressed by canonical IDs bound to a model and a level. Screen positions, pose-table
  indices and display instances are never identities (INV-6, KN1.5). Equality of two jumbling configurations is exact
  pose equality of every labelled piece, with no symmetry quotient (KN1.6).
- **ADR-L04.2** Each puzzle has identity levels joined by exact maps with named kernels. The maps go one way, from finer
  to coarser, and the levels form a partial order: pose → sticker class → region class → {coset → tree data and heights;
  shape}, with labels beside the sticker class and position beside the region class, on the lattice (KN1.7).
- **ADR-L04.3** Digests name their level, and a digest of one level never compares with one of another (KN1.8).
- **ADR-L04.4** Moving to a coarser level is an explicit call that returns a partial result off the lattice, and no
  operation lifts a value to a finer level (KN1.9, KN1.10, LG2.2, LG2.3).
- **ADR-L04.5** Predicates, goals and protection are typed by the level they read. Protection is two predicates, and a
  verdict never crosses between them (KN1.11).
- **ADR-L04.6** Transfer between classes follows the gauge theorem and nothing else. KN1.14 states what transfers,
  KN1.15 says what reads at which level, KN1.16 gives exact transport, and KN1.17 lists what does not transfer. A class
  is never equality.
- **ADR-L04.7** A derived model is a value with a derivation, never a new authoritative puzzle. A claim moves to or from
  it only along a map that preserves the claim's subject (KN1.12, KN1.13).
- **ADR-L04.8** A word's equality key has a type of its own, and it never stands for the literal word. Execution,
  preview, protection, commit and the token all use the literal letters (KN2.7, LG2.13, LG2.14).
- **ADR-L04.9** Every display instance maps back to a canonical ID through a source map bound to its plan generation and
  base revision. Picking resolves instance, then source map, then canonical ID (RPI.20 to RPI.23).

## What this record does not decide

Open whatever the owner answers:

- A complete canonical rewrite for the word key on every menu (LG2.18). The four hypotheses of default 10 bound what the
  key may claim until then.
- A sticker-class digest, which is optional and does not exist yet (KN1.8).
- Multi-layer pick disambiguation (RPI.27), which is a product decision for the design line.

This record answers no owner question.

## Why

- Each level has a demonstrated trap. S_A must not be read as R_A: with R_A, 39,530 labels of a solved state differ
  (KN1.7). A label digest cannot state reachability, because a solved state and the same state with centre 0 turned
  share their labels (EVD.11). Pose-table indices are reordered by `_compact`, so they cannot name anything across a
  revision (KN1.5, KN4.8).
- A chain would have been simpler, and it would be false. Coset and shape are incomparable: two states with equal cosets
  can have different shapes (KN1.7). So the record keeps the partial order.
- Theorem 1 of note N1 gives exact conditions for transfer, including off the lattice. Writing them into KN1 means a
  tool can reuse a result along a gauge with no re-run, and it cannot reuse one anywhere else.
- Types carry this at no cost to the solver. A piece of 600-cell-Full and a piece of the jumbling puzzle do not unify
  even if their stored integers match (LG2.2).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| One level everywhere | Pose is not held for 600-cell-Full, and labels are undefined off the lattice |
| A chain of levels | False: coset and shape do not compare |
| Pose-table indices as IDs | Reordered by `_compact`; they name a table row, not an object |
| Picking by pixels or colour | Screens are not identities (INV-6), and several instances can show one object |
| Execution by normal form | Would commit a word the solver did not see, and it breaks default 10's separation of key and word |

## Consequences

- Tool authors have to name the level of what they compare. Implicit projection is refused, and that costs a line of
  code each time.
- An answer can be partial. Off the lattice a label read reports how many pieces are off, and a program has to handle
  that.
- Equal digests at one level say nothing about another level, so caches and records carry the level.
- Stage 19's family maps transfer legality and shape-level claims (KN1.13). Reachability moves along no gauge (KN1.17).

## Evidence

Class: source. Note N1 and its Theorem 1 are recorded in DT-L1. Under chapter 06's grades N1 stands at (theorem,
producer) until a model review reads it, and its transfer step is the reviewed lemma of stage 19. No person has reviewed
either. The engine test is CT-KN2-06, which does not exist yet. The label-lift conformance test is LP2 acceptance.

## What the owner accepts

Accepting this record accepts ADR-L04.1 to ADR-L04.9 and the [D] clauses they cite.

The owner may overrule these integrator additions: KN1.14 to KN1.17, which come from note N1 and were not in the
partition's first text; shape as a multiset (KN1.7), which corrects note N1's wording for tables that collide; and the
names "sticker class" and "frame group", which replace the first draft's "visible pose class" and "Stab_A".
